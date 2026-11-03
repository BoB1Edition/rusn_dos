// Ver: 5 File: crate/dos/src/bat/runner.rs
//! BAT-интерпретатор уровня COMMAND.COM (вариант B):
//! каждая дочерняя программа получает свежий DosSystem, а состояние оболочки
//! (CWD, текущий диск, окружение, errorlevel, echo) живёт в Rust и переносится
//! в ребёнка перед стартом и читается обратно после завершения.
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use bus::Display;
use bus::peripherals::keyboard::Keyboard;
use emu_config::{EnvironmentConfig, MachineConfig};

use crate::filesystem::DiskDriver;
use crate::loader;
use crate::system::DosSystem;

const NO_STDIN_MSG: &str = "--no_stdin_keys: программа ожидает ввод с клавиатуры; не используйте этот флаг с интерактивными приложениями";

#[derive(Debug, Clone)]
pub struct BatConfig {
    pub max_depth: u16,
    pub no_stdin_keys: bool,
    pub no_log: bool,
}

/// Состояние «оболочки» между запусками программ.
#[derive(Debug, Clone, Default)]
pub struct ShellState {
    pub cwd: HashMap<char, String>,
    pub current_drive: char,
    pub env: HashMap<String, String>,
    pub echo_on: bool,
    pub errorlevel: u8,
}

enum Flow {
    Next,
    Goto(usize),
    Exit(Option<u8>),
}

/// Источник клавиш хоста (консольный мост注入 из бинарного крейта).
pub type PumpFn<'a> = dyn FnMut(&mut Keyboard) -> Result<(), String> + 'a;

fn shell_print(s: &str) {
    println!("{}", s);
    std::io::stdout().flush().ok();
}

fn parse_code(rest: &str) -> Option<u8> {
    rest.trim().parse::<u8>().ok()
}

fn split_first(line: &str) -> (&str, &str) {
    let mut it = line.trim().splitn(2, char::is_whitespace);
    let a = it.next().unwrap_or("");
    let b = it.next().unwrap_or("").trim();
    (a, b)
}

/// Токены с уважением к кавычкам (кавычки сохраняются внутри токена, как в DOS).
fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    for c in s.chars() {
        match c {
            '"' => {
                in_q = !in_q;
                cur.push('"');
            }
            c if c.is_whitespace() && !in_q => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Раскрытие %0..%9 и %VAR% (неизвестная переменная -> пустая строка, %% -> %).
fn expand(line: &str, state: &ShellState, params: &[String]) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] != '%' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        if i + 1 >= chars.len() {
            out.push('%');
            break;
        }
        let c = chars[i + 1];
        if c == '%' {
            out.push('%');
            i += 2;
            continue;
        }
        if c.is_ascii_digit() {
            let idx = (c as u8 - b'0') as usize;
            if let Some(p) = params.get(idx) {
                out.push_str(p);
            }
            i += 2;
            continue;
        }
        if let Some(close) = chars[i + 1..].iter().position(|&ch| ch == '%') {
            let name: String = chars[i + 1..i + 1 + close].iter().collect();
            let val = state
                .env
                .get(&name.to_ascii_uppercase())
                .cloned()
                .unwrap_or_default();
            out.push_str(&val);
            i += 2 + close;
            continue;
        }
        out.push('%');
        i += 1;
    }
    out
}

fn split_dir_name(p: &str) -> (String, String) {
    match p.rsplit_once(|c| c == '\\' || c == '/') {
        Some((d, n)) => (d.to_string(), n.to_string()),
        None => (String::new(), p.to_string()),
    }
}

fn join_dos(base: &str, extra: &str) -> String {
    let mut v: Vec<&str> = Vec::new();
    for part in [base, extra] {
        for c in part.split(|c| c == '\\' || c == '/') {
            if !c.is_empty() {
                v.push(c);
            }
        }
    }
    v.join("\\")
}

/// Регистронезависимый поиск файла в каталоге хоста.
fn find_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    let direct = dir.join(name);
    if direct.is_file() {
        return Some(direct);
    }
    let rd = std::fs::read_dir(dir).ok()?;
    for ent in rd.flatten() {
        if ent.file_name().to_string_lossy().eq_ignore_ascii_case(name) && ent.path().is_file() {
            return Some(ent.path());
        }
    }
    None
}

struct Runner {
    state: ShellState,
    drivers: Vec<DiskDriver>,
    machine_cfg: MachineConfig,
    cfg: BatConfig,
    params: Vec<String>,
    depth: u16,
    pending_cls: bool,
}

/// Точка входа: запуск BAT-файла как «оболочки».
pub fn run_bat(
    host_path: PathBuf,
    args: Vec<String>,
    drivers: Vec<DiskDriver>,
    machine_cfg: MachineConfig,
    env: EnvironmentConfig,
    init_drive: char,
    init_dir: String,
    cfg: BatConfig,
    display: &mut Option<&mut dyn Display>,
    pump: &mut PumpFn,
) -> Result<u8, Box<dyn std::error::Error>> {
    let mut state = ShellState {
        cwd: HashMap::new(),
        current_drive: init_drive,
        env: env.variables.clone(),
        echo_on: true,
        errorlevel: 0,
    };
    state.cwd.insert(
        init_drive,
        init_dir.trim_matches(|c| c == '\\' || c == '/').to_string(),
    );
    let name = host_path
        .file_name()
        .map(|n| n.to_string_lossy().to_uppercase())
        .unwrap_or_default();
    let mut params = vec![name];
    params.extend(args.iter().cloned());
    let mut r = Runner {
        state,
        drivers,
        machine_cfg,
        cfg,
        params: Vec::new(),
        depth: 0,
        pending_cls: false,
    };
    let code = r.run_batch(&host_path, params, 1, display, pump)?;
    Ok(code)
}

impl Runner {
    fn path_entries(&self) -> Vec<(char, String)> {
        self.state
            .env
            .get("PATH")
            .map(|s| s.as_str())
            .unwrap_or("")
            .split(';')
            .filter_map(|e| {
                let e = e.trim();
                if e.len() >= 2 && e.chars().nth(1) == Some(':') {
                    Some((
                        e.chars().next().unwrap().to_ascii_uppercase(),
                        e[2..].trim_matches(|c| c == '\\' || c == '/').to_string(),
                    ))
                } else {
                    None
                }
            })
            .collect()
    }

    fn host_parent_dos(&self, path: &Path) -> Option<(char, String)> {
        for d in &self.drivers {
            let root = PathBuf::from(&d.root_path);
            let (p, r) = match (path.canonicalize(), root.canonicalize()) {
                (Ok(cp), Ok(cr)) => (cp, cr),
                _ => (path.to_path_buf(), root.clone()),
            };
            if let Ok(rel) = p.strip_prefix(&r) {
                let parent = rel.parent()?;
                let dos = parent
                    .to_str()?
                    .replace('/', "\\")
                    .trim_matches(|c| c == '\\' || c == '/')
                    .to_string();
                return Some((d.letter, dos));
            }
        }
        None
    }

    fn resolve_command(&self, token: &str) -> Option<(PathBuf, bool)> {
        let clean = token.trim_matches('"');
        if clean.is_empty() {
            return None;
        }
        // 1) отделяем префикс диска ОТ пути
        let (drive_opt, rest_path) = if clean.len() >= 2 && clean.chars().nth(1) == Some(':') {
            (
                Some(clean.chars().next().unwrap().to_ascii_uppercase()),
                &clean[2..],
            )
        } else {
            (None, clean)
        };
        let (dirpart, namepart) = split_dir_name(rest_path);
        let names: Vec<String> = if namepart.contains('.') {
            let r = vec![namepart.clone()];
            r
        } else {
            let r = vec![
                format!("{}.COM", namepart),
                format!("{}.EXE", namepart),
                format!("{}.BAT", namepart),
            ];
            r
        };
        // 2) базы поиска: диск-абсолют | \-абсолют | cwd+PATH
        let bases: Vec<(char, String)> = if let Some(d) = drive_opt {
            let r = vec![(d, String::new())];
            r
        } else if rest_path.starts_with('\\') || rest_path.starts_with('/') {
            let r = vec![(self.state.current_drive, String::new())];
            r
        } else {
            let mut v = vec![(
                self.state.current_drive,
                self.state
                    .cwd
                    .get(&self.state.current_drive)
                    .cloned()
                    .unwrap_or_default(),
            )];
            v.extend(self.path_entries());
            v
        };
        for (drive, base) in bases {
            let driver = match self.drivers.iter().find(|d| d.letter == drive) {
                Some(d) => d,
                None => continue,
            };
            let rel_dir = join_dos(&base, &dirpart);
            let mut host_dir = driver.root_path.clone();
            if !rel_dir.is_empty() {
                host_dir.push(rel_dir);
            }
            for n in &names {
                if let Some(p) = find_ci(&host_dir, n) {
                    let is_bat = p
                        .extension()
                        .map(|e| e.to_string_lossy().to_ascii_uppercase() == "BAT")
                        .unwrap_or(false);
                    return Some((p, is_bat));
                }
            }
        }
        None
    }

    fn shell_exists(&self, dos_path: &str) -> bool {
        let (drive, rel) = if dos_path.len() >= 2 && dos_path.chars().nth(1) == Some(':') {
            (
                dos_path.chars().next().unwrap().to_ascii_uppercase(),
                dos_path[2..].to_string(),
            )
        } else {
            (
                self.state.current_drive,
                join_dos(
                    self.state
                        .cwd
                        .get(&self.state.current_drive)
                        .map(|s| s.as_str())
                        .unwrap_or(""),
                    dos_path,
                ),
            )
        };
        let driver = match self.drivers.iter().find(|d| d.letter == drive) {
            Some(d) => d,
            None => return false,
        };
        let mut host = driver.root_path.clone();
        let rel = rel.trim_start_matches(|c| c == '\\' || c == '/');
        if !rel.is_empty() {
            host.push(rel);
        }
        host.exists()
    }

    fn run_batch(
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

    fn exec_line(
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

    fn cmd_echo(&mut self, trimmed: &str) {
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

    fn cmd_cd(&mut self, rest: &str) {
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
        for c in pathpart.split(|c| c == '\\' || c == '/') {
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
        self.state.cwd.insert(drive, rel);
    }

    fn cmd_cls(&mut self, display: &mut Option<&mut dyn Display>) {
        if display.is_none() {
            print!("\x1b[2J\x1b[H");
            std::io::stdout().flush().ok();
        } else {
            self.pending_cls = true;
        }
    }

    fn cmd_goto(&self, rest: &str, labels: &HashMap<String, usize>) -> Flow {
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

    fn cmd_if(
        &mut self,
        rest: &str,
        labels: &HashMap<String, usize>,
        if_depth: u8,
        display: &mut Option<&mut dyn Display>,
        pump: &mut PumpFn,
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
        } else if let Some(r) = work.strip_prefix("EXIST ") {
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

    fn cmd_set(&mut self, rest: &str) {
        if rest.is_empty() {
            let mut pairs: Vec<(&String, &String)> = self.state.env.iter().collect();
            pairs.sort_by_key(|(k, _)| k.clone());
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
                } else {
                    self.state.env.insert(k, v.trim().to_string());
                }
            }
            None => shell_print("Environment variable assignment expected"),
        }
    }

    fn cmd_pause(
        &mut self,
        rest: &str,
        display: &mut Option<&mut dyn Display>,
        pump: &mut PumpFn,
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
        let env_cfg = EnvironmentConfig {
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
            }
        }
        self.state.current_drive = ds.filesystem.get_current_drive();
        Ok(code)
    }
}

/// Цикл выполнения дочерней программы. Display принимается по значению и
/// возвращается обратно: время жизни 'd живёт в вызывающем коде.
fn child_loop<'d>(
    ds: &mut DosSystem,
    mut display: Option<&'d mut dyn Display>,
    pump: &mut PumpFn,
    cfg: &BatConfig,
) -> (Result<u8, String>, Option<&'d mut dyn Display>) {
    let result = loop {
        if ds.cpu.halted || ds.program_terminated {
            break Ok(if ds.program_terminated { ds.exit_code } else { 1 });
        }
        if let Err(e) = pump(&mut ds.machine.keyboard) {
            break Err(e);
        }
        // Явный ре-бороу строго внутри аргумента вызова: заимствование живёт
        // ровно до конца step_frame и НЕ растягивается на 'd / на итерации цикла.
        match display.as_mut() {
            Some(d) => ds.step_frame(Some(&mut **d)),
            None => ds.step_frame(None),
        }
        if ds.suspended.is_some() {
            if cfg.no_stdin_keys && display.is_none() {
                break Err(NO_STDIN_MSG.to_string());
            }
            if display.is_none() {
                thread::sleep(Duration::from_millis(10));
            }
        }
    };
    let result = if display.as_ref().map(|d| !d.is_open()).unwrap_or(false) {
        Err("Окно эмулятора закрыто во время работы дочерней программы".into())
    } else {
        result
    };
    (result, display)
}

/// Ожидание клавиши между детьми (pause): консоль через pump, графика через окно.
fn wait_key<'d>(
    mut display: Option<&'d mut dyn Display>,
    pump: &mut PumpFn,
) -> (Result<(), String>, Option<&'d mut dyn Display>) {
    let mut kb = Keyboard::new();
    let result = loop {
        if let Err(e) = pump(&mut kb) {
            break Err(e);
        }
        // То же правило: borrow только на время вызова poll_keys.
        let keys: Vec<(u8, u8)> = match display.as_mut() {
            Some(d) => d.poll_keys(),
            None => Vec::new(),
        };
        for (sc, ch) in keys {
            kb.push_key(sc, ch);
        }
        if kb.pop_key().is_some() {
            break Ok(());
        }
        thread::sleep(Duration::from_millis(10));
    };
    (result, display)
}