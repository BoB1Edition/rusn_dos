// Ver: 2 File: crate/dos/src/loader/mod.rs
pub mod com_loader;
pub mod exe_header;
pub mod exe_loader;

use std::path::PathBuf;
use x86::cpu::X86Cpu;

use crate::system::DosSystem;

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

/// Создаёт CPU с трассировкой опкодов (или без, если `no_log`).
pub(crate) fn make_cpu(no_log: bool) -> Result<X86Cpu, Box<dyn std::error::Error>> {
    if no_log {
        Ok(X86Cpu::new())
    } else {
        Ok(X86Cpu::new_with_tracer("logopcode.txt")?)
    }
}

pub fn load_executable(
    path: PathBuf,
    no_log: bool,
) -> Result<DosSystem, Box<dyn std::error::Error>> {
    match detect_executable_type(&path) {
        ExecutableType::Com => {
            let loader = com_loader::ComLoader::from_file(&path)?;
            loader.exec(no_log)
        }
        ExecutableType::Exe => {
            let mut loader = exe_loader::ExeLoader::from_file(&path)?;
            loader.exec(no_log)
        }
    }
}
