// Ver: 2 File: crate/dos/src/bat/runner/commands.rs

use std::collections::HashMap;
use std::io::Write;

use bus::Display;

use super::flow::wait_key;
use super::types::Flow;
use super::util::shell_print;

impl super::Runner {
    pub(crate) fn cmd_echo(&mut self, trimmed: &str) {
        let up = trimmed.to_ascii_uppercase();
        if up == "ECHO" {
            shell_print(if self.state.echo_on {
                "ECHO is on"
            } else {
                "ECHO is off"
            });
            return;
        }
        if up.starts_with("ECHO.") {
            shell_print(&trimmed[5..]);
            return;
        }
        let r = trimmed[5..].trim();
        if r.eq_ignore_ascii_case("on") {
            self.state.echo_on = true;
        } else if r.eq_ignore_ascii_case("off") {
            self.state.echo_on = false;
        } else {
            shell_print(r);
        }
    }

    pub(crate) fn cmd_cd(&mut self, rest: &str) {
        let arg = rest.trim();
        if arg.is_empty() {
            let d = self.state.current_drive;
            let dir = self.state.cwd.get(&d).cloned().unwrap_or_default();
            shell_print(&format!("{}:\\{}", d, dir));
            return;
        }
        let (drive, pathpart) = if arg.len() >= 2 && arg.chars().nth(1) == Some(':') {
            (
                arg.chars().next().unwrap().to_ascii_uppercase(),
                arg[2..].to_string(),
            )
        } else {
            (self.state.current_drive, arg.to_string())
        };
        if !self.drivers.iter().any(|x| x.letter == drive) {
            shell_print("Invalid drive specification");
            return;
        }
        let base = if pathpart.starts_with('\\') || pathpart.starts_with('/') {
            String::new()
        } else {
            self.state.cwd.get(&drive).cloned().unwrap_or_default()
        };
        let mut comps: Vec<String> = base
            .split('\\')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        for c in pathpart.split(['\\', '/']) {
            match c {
                "" | "." => continue,
                ".." => {
                    comps.pop();
                }
                _ => comps.push(c.to_string()),
            }
        }
        let rel = comps.join("\\");
        let driver = self.drivers.iter().find(|d| d.letter == drive).unwrap();
        let mut host = driver.root_path.clone();
        if !rel.is_empty() {
            host.push(&rel);
        }
        if !host.is_dir() {
            shell_print("Invalid directory");
            return;
        }
        log::debug!("pwd change {}:\\{}", drive, rel);
        self.state.cwd.insert(drive, rel);
    }

    pub(crate) fn cmd_cls(&mut self, display: &mut Option<&mut dyn Display>) {
        if display.is_none() {
            print!("\x1b[2J\x1b[H");
            std::io::stdout().flush().ok();
        } else {
            self.pending_cls = true;
        }
    }

    pub(crate) fn cmd_goto(&self, rest: &str, labels: &HashMap<String, usize>) -> Flow {
        let name = rest
            .trim()
            .trim_start_matches(':')
            .trim()
            .to_ascii_uppercase();
        match labels.get(&name) {
            Some(&i) => Flow::Goto(i),
            None => {
                shell_print(&format!("Goto target not found: {}", name));
                Flow::Exit(None)
            }
        }
    }

    pub(crate) fn cmd_if(
        &mut self,
        rest: &str,
        labels: &HashMap<String, usize>,
        if_depth: u8,
        display: &mut Option<&mut dyn Display>,
        pump: &mut super::types::PumpFn,
    ) -> Result<Flow, String> {
        if if_depth > 4 {
            return Err("Слишком много вложенных IF".into());
        }
        let mut neg = false;
        let mut s: &str = rest;
        let up0 = s.to_ascii_uppercase();
        let mut work: String = up0;
        if let Some(r) = work.strip_prefix("NOT ") {
            neg = true;
            work = r.to_string();
            s = &s[4..];
        }
        let (cond, tail): (bool, &str) = if let Some(r) = work.strip_prefix("ERRORLEVEL ") {
            let digits: String = r.chars().take_while(|c| c.is_ascii_digit()).collect();
            let n: u8 = digits.parse().unwrap_or(0);
            let cond = self.state.errorlevel >= n;
            // хвост берём из исходной строки (сохраняем регистр команды)
            let off = rest.len() - s.len(); // съеденный префикс (NOT )
            let _ = off;
            let tail = &s[digits.len() + "ERRORLEVEL ".len()..];
            (cond, tail)
        } else if let Some(_r) = work.strip_prefix("EXIST ") {
            let rest_orig = &s["EXIST ".len()..];
            let tok: String = rest_orig
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            let cond = self.shell_exists(&tok);
            let tail = &rest_orig[tok.len()..];
            (cond, tail)
        } else {
            shell_print("Unsupported IF condition");
            return Ok(Flow::Next);
        };
        let take = if neg { !cond } else { cond };
        let cmd_tail = tail.trim();
        if !take || cmd_tail.is_empty() {
            return Ok(Flow::Next);
        }
        self.exec_line(cmd_tail, labels, if_depth + 1, display, pump)
    }

    pub(crate) fn cmd_set(&mut self, rest: &str) {
        if rest.is_empty() {
            let mut pairs: Vec<(&String, &String)> = self.state.env.iter().collect();
            pairs.sort_by(|(a, _), (b, _)| a.cmp(b));
            for (k, v) in pairs {
                shell_print(&format!("{}={}", k, v));
            }
            return;
        }
        match rest.split_once('=') {
            Some((k, v)) => {
                let k = k.trim().to_ascii_uppercase();
                if k.is_empty() {
                    return;
                }
                if v.is_empty() {
                    self.state.env.remove(&k);
                    log::debug!("Environment: UNSET {}", k);
                } else {
                    log::debug!("Environment: SET {}={}", k, v.trim());
                    self.state.env.insert(k, v.trim().to_string());
                }
            }
            None => shell_print("Environment variable assignment expected"),
        }
    }

    pub(crate) fn cmd_pause(
        &mut self,
        rest: &str,
        display: &mut Option<&mut dyn Display>,
        pump: &mut super::types::PumpFn,
    ) -> Result<(), String> {
        if rest.is_empty() {
            shell_print("Press any key to continue . . .");
        } else {
            shell_print(rest);
        }
        let disp = display.take();
        let (res, disp) = wait_key(disp, pump);
        *display = disp;
        res
    }
}
