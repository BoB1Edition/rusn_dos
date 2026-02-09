// Ver: 1
use std::{
    error::Error, fs, path::PathBuf
};

use dos_core::loader;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct App {
    resolution: Resolution,
    title: String,
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

impl Default for App {
    fn default() -> Self {
        Self {
            resolution: Resolution::default(),
            title: String::from("rust_dos"),
        }
    }
}

impl App {
    pub fn load_from_file(config: PathBuf) -> Self {
        let data = match fs::read_to_string(&config) {
            Ok(content) => {
                let trimmed = content.trim();
                if trimmed.is_empty() {
                    log::info!("Config file is empty: {}", config.display());
                    return Self::default();
                }
                trimmed.to_owned()
            }
            Err(e) => {
                log::info!("config {} not found: {e}", config.display());
                return Self::default();
            }
        };
        if (data.starts_with('{') && data.ends_with('}'))
            || (data.starts_with('[') && data.ends_with(']'))
        {
            if let Ok(app) = serde_json::from_str(&data) {
                log::info!("Loaded config as JSON from {}", config.display());
                return app;
            } else {
                log::warn!("Failed to parse config as JSON: {}", config.display());
            }
        }

        if let Ok(app) = toml::from_str(&data) {
            log::info!("Loaded config as TOML from {}", config.display());
            return app;
        }

        log::error!(
            "Failed to parse config as either JSON or TOML: {}",
            config.display()
        );
        Self::default()
    }
    pub fn run(&self, program: PathBuf) -> Result<(), Box<dyn Error>> {
        /*let data = fs::read(program)?;
        let dos = DosExecutable::from(data)?;
        let mut dm = dos.exec()?;*/
        let mut dm = loader::load_executable(program)?;
        dm.run();
        Ok(())
    }
}
