// Ver: 3 File: src/app.rs
use std::{
    error::Error,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use config::{self, Config, File};
use dos_core::{DosMachine, loader};
use minifb::{Window, WindowOptions};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct App {
    resolution: Resolution,
    title: String,
    drivers: Vec<DriverConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DriverConfig {
    pub letter: String, // "C", "Z" — преобразуем в char
    pub path: String,
    #[serde(default = "default_true")]
    pub read_only: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
struct Resolution {
    width: u16,
    height: u16,
}

impl Default for Resolution {
    fn default() -> Self {
        Self {
            width: 320,
            height: 240,
        }
    }
}

impl Default for DriverConfig {
    fn default() -> Self {
        Self {
            letter: "Z".to_string(), // "C", "Z" — преобразуем в char
            path: "./".to_string(),
            read_only: true,
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            resolution: Resolution::default(),
            title: String::from("rust_dos"),
            drivers: vec![DriverConfig::default()],
        }
    }
}

impl App {
    fn determine_program_drive_and_dir(&self, program_path: &Path) -> Option<(char, String)> {
        // Получаем абсолютный путь к программе
        let canonical_program = program_path.canonicalize().ok()?;

        // Перебираем все смонтированные диски
        for driver_cfg in &self.drivers {
            let driver_path = PathBuf::from(&driver_cfg.path);
            let canonical_driver = driver_path.canonicalize().ok()?;

            // Проверяем, является ли программа частью этого диска
            if canonical_program.starts_with(&canonical_driver) {
                // Вычисляем относительный путь внутри диска
                let rel_path = canonical_program.strip_prefix(&canonical_driver).ok()?;

                // Удаляем имя файла, оставляем только каталог
                let parent = rel_path.parent()?;

                // Преобразуем в строку в стиле DOS
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

    fn init_drivers(&self, dm: &mut DosMachine) -> Result<(), Box<dyn Error>> {
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

            let driver = dos_core::DiskDriver {
                letter: letter,
                root_path: PathBuf::from(&driver_cfg.path),
                read_only: driver_cfg.read_only,
            };

            dm.filesystem
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
        dm: &mut DosMachine,
        program: &Path,
    ) -> Result<(), Box<dyn Error>> {
        if let Some((drive, dir)) = self.determine_program_drive_and_dir(program) {
            dm.filesystem
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
        //let conf = Config::builder();

        let conf = match Config::builder()
            .add_source(conf_file)
            //.required(false)
            .build()
        {
            Ok(x) => x,
            Err(err) => {
                log::warn!("Error config: {}", err);
                return Self::default();
            }
        };
        let result = match conf.try_deserialize() {
            Ok(x) => x,
            Err(err) => {
                log::warn!("Result config: {}", err);
                return Self::default();
            }
        };
        return result;
    }

    pub fn run(&self, program: PathBuf, no_log: bool) -> Result<(), Box<dyn Error>> {
        let mut dm = loader::load_executable(program.clone(), no_log)?;
        self.init_drivers(&mut dm)?;
        self.setup_program_directory(&mut dm, &program)?;
        if program.exists() && program.is_file() {
            self.setup_program_directory(&mut dm, &program)?;
            dm.run(None)?;
            return Ok(());
        } else {
            let name = program.file_name().unwrap_or(OsStr::new("None")).display();
            return Err(format!("program: {} not found or this no file", name).into());
        }
    }

    pub fn run_with_graphics(&self, program: PathBuf, no_log: bool) -> Result<(), Box<dyn Error>> {
        let mut dm = loader::load_executable(program.clone(), no_log)?;
        self.init_drivers(&mut dm)?;
        self.setup_program_directory(&mut dm, &program)?;
        if program.exists() && program.is_file() {
            let mut window = Window::new(
                &self.title,
                self.resolution.width as usize,
                self.resolution.height as usize,
                WindowOptions::default(),
            )?;
            window.set_target_fps(60);
            //while window.is_open() && !dm.halted() {
                // 1. Выполняем инструкции CPU
                dm.run(Some(&mut window))?;
            //}
            Ok(())
        } else {
            let name = program.file_name().unwrap_or(OsStr::new("None")).display();
            return Err(format!("program: {} not found or this no file", name).into());
        }
    }
}
