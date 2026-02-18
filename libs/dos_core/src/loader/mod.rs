// Ver: 1
pub mod com_loader;
pub mod exe_loader;
pub mod exe_header;

use std::path::PathBuf;
use crate::DosMachine;

pub enum ExecutableType {
    Com,
    Exe,
}

pub fn detect_executable_type(path: &PathBuf) -> ExecutableType {
    let data = std::fs::read(path).unwrap_or_default();
    if data.len() >= 2 && u16::from_le_bytes(data[0..2].try_into().unwrap_or([0; 2])) == 0x5A4D {
        ExecutableType::Exe
    } else {
        ExecutableType::Com
    }
}

pub fn load_executable(path: PathBuf, no_log: bool) -> Result<DosMachine, Box<dyn std::error::Error>> {
    match detect_executable_type(&path) {
        ExecutableType::Com => {
            let loader = com_loader::ComLoader::from_file(&path)?;
            loader.exec(no_log)
        }
        ExecutableType::Exe => {
            let loader = exe_loader::ExeLoader::from_file(&path)?;
            loader.exec(no_log)
        }
    }
}