// Ver: 1 File: crate/dos/src/bat/runner/exec.rs

use std::collections::HashMap;
use std::path::Path;

use bus::Display;

use super::flow::child_loop;
use super::types::{Flow, PumpFn};
use super::util::{expand, parse_code, shell_print, split_first, tokenize};
use crate::loader;

impl super::Runner {
    pub(super) fn run_batch(
        &mut self,
        path: &Path,
        params: Vec<String>,
        depth: u16,
        display: &mut Option<&mut dyn Display>,
        pump: &mut PumpFn,
    ) -> Result<u8, String> {
        if depth > self.cfg.max_depth {
            return Err(format!(
                "Превышена глубина вложенности BAT-файлов: максимум {} (проверьте рекурсивный call)",
                self.cfg.max_depth
            ));
        }
        let data = std::fs::read(path)
            .map_err(|e| format!("Cannot read batch file {}: {}", path.display(), e))?;
        let text = String::from_utf8_lossy(&data);
        let lines: Vec<String> = text
            .lines()
            .enumerate()
            .map(|(i, l)| {
                let l = l.trim_end_matches('\r');
                if i == 0 {
                    l.trim_start_matches('\u{feff}').to_string()
                } else {
                    l.to_string()
                }
            })
            .collect();
        let mut labels: HashMap<String, usize> = HashMap::new();
        for (i, l) in lines.iter().enumerate() {
            let t = l.trim();
            if let Some(rest) = t.strip_prefix(':') {
                labels.entry(rest.trim().to_ascii_uppercase()).or_insert(i);
            }
        }
        log::info!(
            "BAT: start {} (depth {}, {} lines)",
            path.display(),
            depth,
            lines.len()
        );
        let saved_params = std::mem::replace(&mut self.params, params);
        let saved_depth = self.depth;
        self.depth = depth;
        let mut pc = 0usize;
        let mut explicit: Option<u8> = None;
        while pc < lines.len() {
            let raw = lines[pc].clone();
            let (suppress, body) = match raw.trim_start().strip_prefix('@') {
                Some(rest) => (true, rest.to_string()),
                None => (false, raw.clone()),
            };
            let trimmed = body.trim();
            if self.state.echo_on && !suppress && !trimmed.is_empty() {
                shell_print(trimmed);
            }
            if trimmed.is_empty() {
                pc += 1;
                continue;
            }
            let expanded = expand(&body, &self.state, &self.params);
            let flow = self.exec_line(&expanded, &labels, 0, display, pump)?;
            match flow {
                Flow::Next => pc += 1,
                Flow::Goto(i) => pc = i,
                Flow::Exit(code) => {
                    explicit = code;
                    break;
                }
            }
        }
        self.params = saved_params;
        self.depth = saved_depth;
        let code = match explicit {
            Some(c) => {
                self.state.errorlevel = c;
                c
            }
            None => self.state.errorlevel,
        };
        log::info!("BAT: finished {} with code {}", path.display(), code);
        Ok(code)
    }

    pub(super) fn exec_line(
        &mut self,
        line: &str,
        labels: &HashMap<String, usize>,
        if_depth: u8,
        display: &mut Option<&mut dyn Display>,
        pump: &mut PumpFn,
    ) -> Result<Flow, String> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(':') {
            return Ok(Flow::Next);
        }
        let (cmd, rest) = split_first(trimmed);
        let up = cmd.to_ascii_uppercase();
        let up_trimmed = trimmed.to_ascii_uppercase();
        if up_trimmed == "ECHO"
            || up_trimmed.starts_with("ECHO.")
            || up_trimmed.starts_with("ECHO ")
        {
            self.cmd_echo(trimmed);
            return Ok(Flow::Next);
        }
        match up.as_str() {
            "REM" => Ok(Flow::Next),
            "CD" | "CHDIR" => {
                self.cmd_cd(rest);
                Ok(Flow::Next)
            }
            "CLS" => {
                self.cmd_cls(display);
                Ok(Flow::Next)
            }
            "GOTO" => Ok(self.cmd_goto(rest, labels)),
            "IF" => self.cmd_if(rest, labels, if_depth, display, pump),
            "CALL" => {
                if rest.trim_start().starts_with(':') {
                    shell_print("CALL :label (подпрограммы) не поддерживаются на Этапе 4");
                    return Ok(Flow::Next);
                }
                self.run_command(rest, display, pump)?;
                Ok(Flow::Next)
            }
            "SET" => {
                self.cmd_set(rest);
                Ok(Flow::Next)
            }
            "PAUSE" => {
                self.cmd_pause(rest, display, pump)?;
                Ok(Flow::Next)
            }
            "EXIT" => Ok(Flow::Exit(parse_code(rest))),
            _ => {
                // Смена диска: "C:"
                if up.len() == 2 && up.ends_with(':') && rest.is_empty() {
                    let d = up.chars().next().unwrap();
                    if self.drivers.iter().any(|x| x.letter == d) {
                        self.state.current_drive = d;
                        log::debug!(
                            "pwd change {}:{}",
                            d,
                            self.state.cwd.get(&d).map(|s| s.as_str()).unwrap_or("\\")
                        );
                    } else {
                        shell_print("Invalid drive specification");
                    }
                    return Ok(Flow::Next);
                }
                self.run_command(trimmed, display, pump)?;
                Ok(Flow::Next)
            }
        }
    }

    fn run_command(
        &mut self,
        line: &str,
        display: &mut Option<&mut dyn Display>,
        pump: &mut PumpFn,
    ) -> Result<(), String> {
        let (token, rest) = split_first(line);
        let args = tokenize(rest);
        match self.resolve_command(token) {
            Some((path, true)) => {
                let mut params = vec![
                    path.file_name()
                        .map(|n| n.to_string_lossy().to_uppercase())
                        .unwrap_or_default(),
                ];
                params.extend(args);
                let code = self.run_batch(&path, params, self.depth + 1, display, pump)?;
                self.state.errorlevel = code;
            }
            Some((path, false)) => {
                let code = self.run_external(&path, args, display, pump)?;
                self.state.errorlevel = code;
            }
            None => shell_print("Bad command or file name"),
        }
        Ok(())
    }

    fn run_external(
        &mut self,
        path: &Path,
        args: Vec<String>,
        display: &mut Option<&mut dyn Display>,
        pump: &mut PumpFn,
    ) -> Result<u8, String> {
        let env_cfg = emu_config::EnvironmentConfig {
            variables: self.state.env.clone(),
        };
        let mut ds = loader::load_executable(
            path.to_path_buf(),
            self.cfg.no_log,
            &self.machine_cfg,
            &env_cfg,
            args,
        )
        .map_err(|e| e.to_string())?;
        for d in &self.drivers {
            let _ = ds.filesystem.add_driver(d.clone());
        }
        for (drive, dir) in self.state.cwd.clone() {
            let _ = ds.filesystem.set_current_directory(drive, &dir);
        }
        ds.filesystem.set_current_drive(self.state.current_drive);
        ds.filesystem
            .set_search_path_from_env(self.state.env.get("PATH").map(|s| s.as_str()).unwrap_or(""));
        if let Some((drive, dir)) = self.host_parent_dos(path) {
            ds.filesystem.set_program_location(drive, &dir);
        }
        if self.pending_cls {
            for cell in ds.machine.video.text_buffer.iter_mut() {
                *cell = 0x0720;
            }
            ds.machine.video.cursor_row = 0;
            ds.machine.video.cursor_col = 0;
            ds.machine.video.dirty = true;
            self.pending_cls = false;
        }
        let disp = display.take();
        let (code, disp) = child_loop(&mut ds, disp, pump, &self.cfg);
        *display = disp;
        let code = code?;
        for d in &self.drivers {
            if let Some(dir) = ds.filesystem.get_current_directory(d.letter) {
                self.state.cwd.insert(d.letter, dir.to_string());
                log::debug!("pwd change {}:\\{} (from child)", d.letter, dir);
            }
        }
        self.state.current_drive = ds.filesystem.get_current_drive();
        log::debug!("pwd change {} (drive from child)", self.state.current_drive);
        Ok(code)
    }
}
