// Ver: 1 File: crate/dos/src/filesystem/mod.rs
use bus::Machine;
use std::{collections::HashMap, fs, path::PathBuf};

mod device;
mod find;
mod path;
mod search;
mod types;

pub use find::system_time_to_dos_datetime;
pub use path::resolve_device;
pub use types::{DeviceKind, DiskDriver, FileHandle, FoundFile};

use types::SearchContext;

/// Многодисковая файловая система DOS поверх каталогов хоста.
///
/// Хранит состояние эмуляции: смонтированные диски (`drivers`), текущие
/// каталоги по каждому диску, стандартные устройства DOS (CON/NUL/PRN/AUX/
/// COMx/LPTx/CLOCK$/EMMXXXX0), открытые дескрипторы и контексты поиска DTA.
#[derive(Debug)]
pub struct FileSystem {
    drivers: HashMap<char, DiskDriver>,
    current_directories: HashMap<char, String>,
    current_drive: char,
    open_files: HashMap<u16, FileHandle>,
    next_handle: u16,
    search_contexts: HashMap<u32, SearchContext>,
    handle_paths: HashMap<u16, PathBuf>,
    program_drive: char,
    program_dir: String,
    search_path: Vec<(char, String)>,
}

impl FileSystem {
    pub fn new() -> Self {
        let mut fs = Self {
            drivers: HashMap::new(),
            open_files: HashMap::new(),
            next_handle: 5,
            current_directories: HashMap::new(),
            current_drive: 'C',
            search_contexts: HashMap::new(),
            handle_paths: HashMap::new(),
            program_drive: 'C',
            program_dir: String::new(),
            search_path: Vec::new(),
        };
        fs.open_files.insert(0, FileHandle::Device(DeviceKind::Con)); // stdin
        fs.open_files.insert(1, FileHandle::Device(DeviceKind::Con)); // stdout
        fs.open_files.insert(2, FileHandle::Device(DeviceKind::Con)); // stderr
        fs.open_files
            .insert(3, FileHandle::Device(DeviceKind::Com(1))); // aux
        fs.open_files
            .insert(4, FileHandle::Device(DeviceKind::Lpt(1))); // prn

        fs
    }

    pub fn add_driver(&mut self, driver: DiskDriver) -> Result<(), String> {
        if !driver.letter.is_ascii_alphabetic() {
            return Err(format!("Invalid drive letter: {}", driver.letter));
        }
        let letter = driver.letter.to_ascii_uppercase();
        if !driver.root_path.exists() {
            fs::create_dir_all(&driver.root_path)
                .map_err(|e| format!("Failed to create driver path: {}", e))?;
        }
        self.drivers.insert(letter, driver);
        self.current_directories.insert(letter, String::new());
        Ok(())
    }

    pub fn set_current_drive(&mut self, drive_letter: char) {
        self.current_drive = drive_letter.to_ascii_uppercase();
    }

    pub fn get_current_drive(&self) -> char {
        self.current_drive
    }

    pub fn set_current_directory(&mut self, drive_letter: char, path: &str) -> Result<(), String> {
        let letter = drive_letter.to_ascii_uppercase();
        if !self.drivers.contains_key(&letter) {
            return Err(format!("Drive {} not mounted", letter));
        }
        let normalized = path.trim_matches(['\\', '/']).replace('/', "\\");
        self.current_directories.insert(letter, normalized);
        Ok(())
    }

    pub fn get_current_directory(&self, drive_letter: char) -> Option<&str> {
        self.current_directories
            .get(&drive_letter.to_ascii_uppercase())
            .map(|s| s.as_str())
    }

    /// Читает ASCIZ-строку имени файла по адресу DS:DX из памяти машины.
    pub fn extract_filename(&self, ds: u16, dx: u16, machine: &dyn Machine) -> String {
        let mut bytes = Vec::new();
        let base = ((ds as u32) << 4).wrapping_add(dx as u32);
        for i in 0..255u32 {
            let byte = machine.read_mem_u8(base.wrapping_add(i));
            if byte == 0 {
                break;
            }
            bytes.push(byte);
        }
        String::from_utf8_lossy(&bytes).to_string()
    }

    pub fn resolve_path(&self, dos_path: &str) -> Result<PathBuf, String> {
        path::resolve_path(self, dos_path)
    }

    pub fn is_read_only(&self, drive_letter: char) -> bool {
        self.drivers
            .get(&drive_letter.to_ascii_uppercase())
            .is_none_or(|d| d.read_only)
    }

    pub fn handle_path(&self, handle: u16) -> Option<&PathBuf> {
        self.handle_paths.get(&handle)
    }
}

impl Default for FileSystem {
    fn default() -> Self {
        Self::new()
    }
}
