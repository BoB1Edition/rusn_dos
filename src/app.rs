// Ver: 6 File: ./src/app.rs
use std::{
    error::Error,
    ffi::OsStr,
    path::{Path, PathBuf},
};
use bus::Display;
use config::{self, Config, File};
use dos::{DiskDriver, DosSystem, loader};
use emu_config::{DriverConfig, MachineConfig, Resolution, SoundConfig, VideoConfig};
use gui::EmuWindow;
use serde::{Deserialize, Serialize};

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
        }
    }
}

impl App {
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
            let path = dos_workdir_raw[2..].trim_matches(|c| c == '\\' || c == '/').to_string();
            (d, path)
        } else {
            let drive = self.drivers.first().map(|d| d.letter.chars().next().unwrap_or('C')).unwrap_or('C');
            let path = dos_workdir_raw.trim_matches(|c| c == '\\' || c == '/').to_string();
            (drive, path)
        }
    }

    pub fn run(
        &self,
        program: PathBuf,
        _startdir: PathBuf,
        workdir: PathBuf,
        no_log: bool,
        no_stdin_keys: bool,
        args: Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        if !(program.exists() && program.is_file()) {
            let name = program.file_name().unwrap_or(OsStr::new("None")).display();
            return Err(format!("program: {} not found or this no file", name).into());
        }
        let mut ds = loader::load_executable(program.clone(), no_log, &self.machine, args)?;
        self.init_drivers(&mut ds)?;
        self.setup_program_directory(&mut ds, &program)?;
        let (target_drive, dos_path) = self.parse_workdir(&workdir);
        if !dos_path.is_empty() && dos_path != "." {
            match ds.filesystem.set_current_directory(target_drive, &dos_path) {
                Ok(_) => log::info!("DOS working directory overridden to: {}:\\{}", target_drive, dos_path),
                Err(e) => log::warn!("Failed to set DOS working directory to {}:{}: {}", target_drive, dos_path, e),
            }
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
                    return Err(
                        "--no_stdin_keys: программа ожидает ввод с клавиатуры; не используйте этот флаг с интерактивными приложениями"
                            .into(),
                    );
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        if ds.program_terminated {
            log::info!("Program exited with code: {}", ds.exit_code);
        }
        Ok(())
    }

    pub fn run_with_graphics(
        &self,
        program: PathBuf,
        _startdir: PathBuf,
        workdir: PathBuf,
        no_log: bool,
        args: Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        if !(program.exists() && program.is_file()) {
            let name = program.file_name().unwrap_or(OsStr::new("None")).display();
            return Err(format!("program: {} not found or this no file", name).into());
        }
        let mut ds = loader::load_executable(program.clone(), no_log, &self.machine, args)?;
        self.init_drivers(&mut ds)?;
        
        self.setup_program_directory(&mut ds, &program)?;
        
        let (target_drive, dos_path) = self.parse_workdir(&workdir);
        if !dos_path.is_empty() && dos_path != "." {
            match ds.filesystem.set_current_directory(target_drive, &dos_path) {
                Ok(_) => log::info!("DOS working directory overridden to: {}:\\{}", target_drive, dos_path),
                Err(e) => log::warn!("Failed to set DOS working directory to {}:{}: {}", target_drive, dos_path, e),
            }
        }
        
        let mut window = EmuWindow::new(
            &self.title,
            self.resolution.width as usize,
            self.resolution.height as usize,
        )?;
        window.set_target_fps(60);
        
        while window.is_open() && !ds.cpu.halted && !ds.program_terminated {
            ds.step_frame(Some(&mut window));
        }
        
        if ds.program_terminated {
            log::info!("Program exited with code: {}", ds.exit_code);
        }
        Ok(())
    }
}