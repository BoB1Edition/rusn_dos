// Ver: 11 File: ./src/app.rs
use bus::Display;
use bus::peripherals::keyboard::Keyboard;
use config::{self, Config, File};
use dos::bat::BatConfig;
use dos::{DiskDriver, DosSystem, loader};
use emu_config::{
    DriverConfig, EnvironmentConfig, MachineConfig, Resolution, SoundConfig, VideoConfig,
};
use gui::EmuWindow;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct App {
    #[serde(default = "default_title")]
    pub title: String,
    #[serde(default)]
    pub resolution: Resolution,
    #[serde(default)]
    pub machine: MachineConfig,
    #[serde(default)]
    pub video: VideoConfig,
    #[serde(default)]
    pub sound: SoundConfig,
    #[serde(default)]
    pub drivers: Vec<DriverConfig>,
    #[serde(default)]
    pub environment: EnvironmentConfig,
}

fn default_title() -> String {
    "rust_dos".to_string()
}

impl Default for App {
    fn default() -> Self {
        Self {
            title: default_title(),
            resolution: Resolution::default(),
            machine: MachineConfig::default(),
            video: VideoConfig::default(),
            sound: SoundConfig::default(),
            drivers: vec![DriverConfig::default()],
            environment: EnvironmentConfig::default(),
        }
    }
}

impl App {
    fn make_drivers(&self) -> Vec<DiskDriver> {
        self.drivers
            .iter()
            .filter_map(|c| {
                c.letter.chars().next().map(|l| DiskDriver {
                    letter: l.to_ascii_uppercase(),
                    root_path: PathBuf::from(&c.path),
                    read_only: c.read_only,
                })
            })
            .collect()
    }
    fn is_bat(p: &Path) -> bool {
        let result = matches!(
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_uppercase())
                .as_deref(),
            Some("BAT") | Some("CMD")
        );
        result
    }

    /// Начальный диск/каталог оболочки: каталог программы, поверх — workdir.
    fn initial_dos_dir(&self, program: &Path, workdir: &Path) -> (char, String) {
        let (mut drive, mut dir) = self
            .determine_program_drive_and_dir(program)
            .unwrap_or(('C', String::new()));
        let (wd, wdir) = self.parse_workdir(workdir);
        if !wdir.is_empty() && wdir != "." {
            drive = wd;
            dir = wdir;
        }
        (drive, dir)
    }

    pub fn resolve_program_path(&self, program: &Path, startdir: &Path) -> Option<PathBuf> {
        let prog_str = program.to_string_lossy().to_string();
        // 0) программа задана DOS-путём: c:\TASM\BIN\TASM.EXE
        if prog_str.contains('\\') || prog_str.len() >= 2 && prog_str.chars().nth(1) == Some(':') {
            if let Some(p) = self.dos_path_to_host(&prog_str) {
                if p.is_file() {
                    return Some(p);
                }
            }
        }
        // 1) обычный host-путь
        if program.exists() && program.is_file() {
            return Some(program.to_path_buf());
        }
        let name = program.file_name()?.to_string_lossy().to_string();
        // 2) startdir (host- или DOS-путь) + имя программы
        let start_host = self
            .dos_path_to_host(&startdir.to_string_lossy())
            .or_else(|| {
                if startdir.is_dir() {
                    Some(startdir.to_path_buf())
                } else {
                    None
                }
            });
        if let Some(sh) = start_host {
            if let Some(p) = Self::join_ci(&sh, &name) {
                return Some(p);
            }
        }
        let names: Vec<String> = if name.contains('.') {
            let r = vec![name];
            r
        } else {
            let r = vec![
                format!("{}.COM", name),
                format!("{}.EXE", name),
                format!("{}.BAT", name),
            ];
            r
        };
        for (letter, dos_dir) in self.environment.path_entries() {
            let cfg = if let Some(x) = self.drivers.iter().find(|d| {
                d.letter.chars().next().map(|c| c.to_ascii_uppercase()) == Some(letter)
            }) {
                x
            } else {
                continue;
            };
            let mut host_dir = PathBuf::from(&cfg.path);
            if !dos_dir.is_empty() {
                host_dir.push(&dos_dir);
            }
            for n in &names {
                if let Some(p) = Self::join_ci(&host_dir, n) {
                    return Some(p);
                }
            }
        }
        None
    }

    fn determine_program_drive_and_dir(&self, program_path: &Path) -> Option<(char, String)> {
        let canonical_program = program_path.canonicalize().ok()?;
        for driver_cfg in &self.drivers {
            let driver_path = PathBuf::from(&driver_cfg.path);
            let canonical_driver = driver_path.canonicalize().ok()?;
            if canonical_program.starts_with(&canonical_driver) {
                let rel_path = canonical_program.strip_prefix(&canonical_driver).ok()?;
                let parent = rel_path.parent()?;
                let dos_path = parent
                    .to_str()?
                    .replace('/', "\\")
                    .trim_matches(|c| c == '\\' || c == '/')
                    .to_string();
                let drive_letter = driver_cfg
                    .letter
                    .chars()
                    .next()
                    .unwrap_or('C')
                    .to_ascii_uppercase();
                return Some((drive_letter, dos_path));
            }
        }
        None
    }

    fn init_drivers(&self, ds: &mut DosSystem) -> Result<(), Box<dyn Error>> {
        for driver_cfg in &self.drivers {
            let letter = match driver_cfg
                .letter
                .chars()
                .next()
                .ok_or_else(|| format!("Invalid drive letter: {}", driver_cfg.letter))
            {
                Ok(x) => x,
                Err(err) => {
                    log::error!("letter invalid: {}", err);
                    return Err(err.into());
                }
            };
            let driver = DiskDriver {
                letter,
                root_path: PathBuf::from(&driver_cfg.path),
                read_only: driver_cfg.read_only,
            };
            ds.filesystem
                .add_driver(driver)
                .map_err(|e| format!("Failed to add driver {}: {}", driver_cfg.letter, e))?;
            log::info!(
                "Mounted drive {}:{} (read_only={})",
                driver_cfg.letter,
                driver_cfg.path,
                driver_cfg.read_only
            );
        }
        Ok(())
    }

    fn setup_program_directory(
        &self,
        ds: &mut DosSystem,
        program: &Path,
    ) -> Result<(), Box<dyn Error>> {
        if let Some((drive, dir)) = self.determine_program_drive_and_dir(program) {
            ds.filesystem
                .set_current_directory(drive, &dir)
                .map_err(|e| format!("Failed to set current directory: {}", e))?;
            ds.filesystem.set_program_location(drive, &dir);
            log::info!("Program launched from {}:\\{}\\", drive, dir);
        } else {
            log::warn!("Could not determine program drive/directory, using root");
        }
        Ok(())
    }

    pub fn load_from_file(config: PathBuf) -> Self {
        let conf_file = File::with_name(config.to_str().unwrap_or("config.toml"));
        let conf = match Config::builder().add_source(conf_file).build() {
            Ok(x) => x,
            Err(err) => {
                log::warn!("Error config: {}", err);
                return Self::default();
            }
        };
        match conf.try_deserialize() {
            Ok(x) => x,
            Err(err) => {
                log::warn!("Result config: {}", err);
                Self::default()
            }
        }
    }

    /// Вспомогательная функция для парсинга workdir (отделение буквы диска от пути)
    fn parse_workdir(&self, workdir: &Path) -> (char, String) {
        let dos_workdir_raw = workdir.to_string_lossy().replace('/', "\\");
        if dos_workdir_raw.len() >= 2 && dos_workdir_raw.chars().nth(1) == Some(':') {
            let d = dos_workdir_raw.chars().next().unwrap().to_ascii_uppercase();
            let path = dos_workdir_raw[2..]
                .trim_matches(|c| c == '\\' || c == '/')
                .to_string();
            (d, path)
        } else {
            let drive = self
                .drivers
                .first()
                .map(|d| d.letter.chars().next().unwrap_or('C'))
                .unwrap_or('C');
            let path = dos_workdir_raw
                .trim_matches(|c| c == '\\' || c == '/')
                .to_string();
            (drive, path)
        }
    }

    pub fn run(
        &self,
        program: PathBuf,
        startdir: PathBuf,
        workdir: PathBuf,
        no_log: bool,
        no_stdin_keys: bool,
        max_call_depth: u16,
        args: Vec<String>,
    ) -> Result<u8, Box<dyn Error>> {
        let program = self
            .resolve_program_path(&program, &startdir)
            .unwrap_or(program);
        if !(program.exists() && program.is_file()) {
            let name = program.file_name().unwrap_or(OsStr::new("None")).display();
            return Err(format!("program: {} not found or this no file", name).into());
        }
        if Self::is_bat(&program) {
            let (drive, dir) = self.initial_dos_dir(&program, &workdir);
            let _raw = if !no_stdin_keys {
                crate::stdin_bridge::RawTerminal::try_enable()
            } else {
                None
            };
            let mut pump = |kb: &mut Keyboard| -> Result<(), String> {
                if _raw.is_some() {
                    crate::stdin_bridge::poll_keys(kb);
                }
                Ok(())
            };
            let mut display: Option<&mut dyn Display> = None;
            let code = dos::bat::run_bat(
                program,
                args,
                self.make_drivers(),
                self.machine.clone(),
                self.environment.clone(),
                drive,
                dir,
                BatConfig {
                    max_depth: max_call_depth,
                    no_stdin_keys,
                    no_log,
                },
                &mut display,
                &mut pump,
            )?;
            log::info!("Batch exited with code: {}", code);
            return Ok(code);
        }
        // === обычный exe/com (тело Ver 8, возврат кода) ===
        let mut ds = loader::load_executable(
            program.clone(),
            no_log,
            &self.machine,
            &self.environment,
            args,
        )?;
        for d in self.make_drivers() {
            ds.filesystem.add_driver(d)?;
        }
        ds.filesystem
            .set_search_path_from_env(self.environment.get("PATH").unwrap_or(""));
        self.setup_program_directory(&mut ds, &program)?;
        let (target_drive, dos_path) = self.parse_workdir(&workdir);
        if !dos_path.is_empty() && dos_path != "." {
            let _ = ds.filesystem.set_current_directory(target_drive, &dos_path);
        }
        let _raw = if !no_stdin_keys {
            crate::stdin_bridge::RawTerminal::try_enable()
        } else {
            None
        };
        loop {
            if ds.cpu.halted || ds.program_terminated {
                break;
            }
            if _raw.is_some() {
                crate::stdin_bridge::poll_keys(&mut ds.machine.keyboard);
            }
            ds.step_frame(None);
            if ds.suspended.is_some() {
                if no_stdin_keys {
                    return Err(crate::stdin_bridge::NO_STDIN_KEYS_ERR.into());
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        if ds.program_terminated {
            log::info!("Program exited with code: {}", ds.exit_code);
        }
        Ok(ds.exit_code)
    }

    pub fn run_with_graphics(
        &self,
        program: PathBuf,
        startdir: PathBuf,
        workdir: PathBuf,
        no_log: bool,
        _no_stdin_keys: bool,
        max_call_depth: u16,
        args: Vec<String>,
    ) -> Result<u8, Box<dyn Error>> {
        let program = self
            .resolve_program_path(&program, &startdir)
            .unwrap_or(program);
        if !(program.exists() && program.is_file()) {
            let name = program.file_name().unwrap_or(OsStr::new("None")).display();
            return Err(format!("program: {} not found or this no file", name).into());
        }
        let mut window = EmuWindow::new(
            &self.title,
            self.resolution.width as usize,
            self.resolution.height as usize,
        )?;
        window.set_target_fps(60);
        if Self::is_bat(&program) {
            let (drive, dir) = self.initial_dos_dir(&program, &workdir);
            let mut pump = |_kb: &mut Keyboard| -> Result<(), String> { Ok(()) };
            let mut display: Option<&mut dyn Display> = Some(&mut window);
            let code = dos::bat::run_bat(
                program,
                args,
                self.make_drivers(),
                self.machine.clone(),
                self.environment.clone(),
                drive,
                dir,
                BatConfig {
                    max_depth: max_call_depth,
                    no_stdin_keys: false,
                    no_log,
                },
                &mut display,
                &mut pump,
            )?;
            log::info!("Batch exited with code: {}", code);
            return Ok(code);
        }
        let mut ds = loader::load_executable(
            program.clone(),
            no_log,
            &self.machine,
            &self.environment,
            args,
        )?;
        for d in self.make_drivers() {
            ds.filesystem.add_driver(d)?;
        }
        ds.filesystem
            .set_search_path_from_env(self.environment.get("PATH").unwrap_or(""));
        self.setup_program_directory(&mut ds, &program)?;
        let (target_drive, dos_path) = self.parse_workdir(&workdir);
        if !dos_path.is_empty() && dos_path != "." {
            let _ = ds.filesystem.set_current_directory(target_drive, &dos_path);
        }
        while !ds.cpu.halted && !ds.program_terminated {
            if !window.is_open() {
                break;
            }
            ds.step_frame(Some(&mut window));
        }
        if ds.program_terminated {
            log::info!("Program exited with code: {}", ds.exit_code);
        }
        Ok(ds.exit_code)
    }

    fn dos_path_to_host(&self, dos: &str) -> Option<PathBuf> {
        let normalized = dos.replace('/', "\\");
        if !(normalized.len() >= 2 && normalized.chars().nth(1) == Some(':')) {
            return None; // без диска не транслируем
        }
        let letter = normalized.chars().next().unwrap().to_ascii_uppercase();
        let cfg = self
            .drivers
            .iter()
            .find(|d| d.letter.chars().next().map(|c| c.to_ascii_uppercase()) == Some(letter))?;
        let mut host = PathBuf::from(&cfg.path);
        for comp in normalized[2..].split('\\').filter(|s| !s.is_empty()) {
            let direct = host.join(comp);
            if direct.exists() {
                host = direct;
            } else {
                let rd = std::fs::read_dir(&host).ok()?;
                let found = rd
                    .flatten()
                    .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(comp))?;
                host = found.path();
            }
        }
        Some(host)
    }

    fn join_ci(dir: &Path, name: &str) -> Option<PathBuf> {
        let direct = dir.join(name);
        if direct.is_file() {
            return Some(direct);
        }
        let rd = std::fs::read_dir(dir).ok()?;
        for ent in rd.flatten() {
            if ent.file_name().to_string_lossy().eq_ignore_ascii_case(name) && ent.path().is_file()
            {
                return Some(ent.path());
            }
        }
        None
    }
}
