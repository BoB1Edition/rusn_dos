// Ver: 1
use std::{
    collections::HashMap,
    fs::{self, File as StdFile, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

#[derive(Debug, Clone)]
pub struct DiskDriver {
    pub letter: char,
    pub root_path: PathBuf,
    pub read_only: bool,
}

#[derive(Debug)]
pub struct FileSystem {
    drivers: HashMap<char, DiskDriver>,
    current_directories: HashMap<char, String>,
    open_files: HashMap<u16, StdFile>, // дескриптор → файл
    next_handle: u16,                  // следующий доступный дескриптор (начиная с 5)
}

impl FileSystem {
    pub fn new() -> Self {
        Self {
            drivers: HashMap::new(),
            open_files: HashMap::new(),
            next_handle: 5, // 0-4 = стандартные дескрипторы (stdin, stdout, stderr, AUX, PRN)
            current_directories: HashMap::new(),
        }
    }

    pub fn add_driver(&mut self, driver: DiskDriver) -> Result<(), String> {
        if !driver.letter.is_ascii_alphabetic() {
            return Err(format!("Invalid drive letter: {}", driver.letter));
        }
        let letter = driver.letter.to_ascii_uppercase();

        // Создаём директорию если её нет
        if !driver.root_path.exists() {
            fs::create_dir_all(&driver.root_path).map_err(|e| {
                format!("Failed to create driver path {:?}: {}", driver.root_path, e)
            })?;
        }

        self.drivers.insert(letter, driver);
        self.current_directories.insert(letter, String::new());
        Ok(())
    }

    pub fn set_current_directory(&mut self, drive_letter: char, path: &str) -> Result<(), String> {
        let letter = drive_letter.to_ascii_uppercase();
        if !self.drivers.contains_key(&letter) {
            return Err(format!("Drive {} not mounted", letter));
        }

        // Нормализация пути: удаляем лишние слеши, приводим к единому формату
        let normalized = path
            .trim_start_matches(|c| c == '\\' || c == '/')
            .trim_end_matches(|c| c == '\\' || c == '/')
            .replace('/', "\\");

        log::info!(
            "Current directory for drive {} set to: {}",
            letter,
            normalized
        );
        self.current_directories.insert(letter, normalized);
        Ok(())
    }

    /// Извлекает имя файла из памяти по указателю DS:DX (ASCIIZ строка)
    pub fn extract_filename(&self, ds: u16, dx: u16, read_u8: impl Fn(u16, u16) -> u8) -> String {
        let mut bytes = Vec::new();
        for i in 0..255 {
            let byte = read_u8(ds, dx.wrapping_add(i as u16));
            if byte == 0 {
                break;
            }
            bytes.push(byte);
        }
        String::from_utf8_lossy(&bytes).to_string()
    }

    /// Преобразует путь в стиле DOS в локальный путь (с безопасной нормализацией)
    pub fn resolve_path(&self, dos_path: &str) -> Result<PathBuf, String> {
        // 1. Если путь НЕ содержит буквы диска — добавляем текущий диск и текущую директорию
        let full_path = if !dos_path.contains(':') {
            // Определяем текущий диск (по умолчанию 'C')
            let current_drive = 'C';

            // Получаем текущую директорию для диска
            let current_dir = self
                .current_directories
                .get(&current_drive)
                .map(|s| s.as_str())
                .unwrap_or("");

            // Формируем полный путь в стиле DOS
            if current_dir.is_empty() {
                format!("{}:\\{}", current_drive, dos_path)
            } else {
                format!("{}:\\{}\\{}", current_drive, current_dir, dos_path)
            }
        } else {
            dos_path.to_string()
        };

        // 2. Стандартная обработка пути с буквой диска
        let parts: Vec<&str> = full_path.split(':').collect();
        if parts.len() < 2 {
            return Err(format!("Invalid DOS path: {}", full_path));
        }

        let drive_letter = parts[0]
            .chars()
            .next()
            .ok_or_else(|| format!("Empty drive letter in path: {}", full_path))?
            .to_ascii_uppercase();

        let driver = self
            .drivers
            .get(&drive_letter)
            .ok_or_else(|| format!("Drive {} not configured", drive_letter))?;

        let mut path_str = parts[1].trim_start_matches(|c| c == '\\' || c == '/');
        if path_str.is_empty() {
            path_str = ".";
        }

        let normalized = path_str.replace('\\', "/");
        let mut local_path = driver.root_path.clone();

        for component in normalized.split('/') {
            match component {
                "" | "." => continue,
                ".." => {
                    if local_path == driver.root_path {
                        return Err(format!("Path traversal blocked: {}", full_path));
                    }
                    local_path.pop();
                }
                _ => local_path.push(component),
            }
        }

        let canonical_root = driver
            .root_path
            .canonicalize()
            .map_err(|e| format!("Canonicalize root failed: {}", e))?;
        let canonical_path = local_path
            .canonicalize()
            .map_err(|e| format!("Canonicalize path failed: {}", e))?;

        if !canonical_path.starts_with(&canonical_root) {
            return Err(format!("Path traversal blocked: {}", full_path));
        }

        Ok(canonical_path)
    }

    pub fn is_read_only(&self, drive_letter: char) -> bool {
        self.drivers
            .get(&drive_letter.to_ascii_uppercase())
            .map_or(true, |d| d.read_only)
    }

    pub fn open_file(&mut self, dos_path: &str, access_mode: u8) -> Result<u16, String> {
        let local_path = self.resolve_path(dos_path)?;

        if access_mode == 0 && !local_path.exists() {
            return Err(format!("File not found: {}", dos_path));
        }
        if local_path.is_dir() {
            return Err(format!("Path is a directory: {}", dos_path));
        }

        // Проверяем права на запись
        let drive_letter = dos_path.chars().next().unwrap_or('C');
        if (access_mode == 1 || access_mode == 2) && self.is_read_only(drive_letter) {
            return Err(format!("Attempt to write to read-only drive: {}", dos_path));
        }

        // Открываем файл
        let file = match access_mode {
            0 => StdFile::open(&local_path), // Только чтение
            1 => OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&local_path), // Только запись (создаёт/перезаписывает)
            2 => OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&local_path), // Чтение+запись
            _ => Err(std::io::Error::from(std::io::ErrorKind::InvalidInput)),
        }
        .map_err(|e| format!("Failed to open file '{}': {}", local_path.display(), e))?;

        // Выделяем дескриптор
        let handle = self.next_handle;
        self.next_handle = self.next_handle.wrapping_add(1);
        if self.next_handle < 5 {
            self.next_handle = 5; // защита от переполнения
        }

        self.open_files.insert(handle, file);
        Ok(handle)
    }

    pub fn close_file(&mut self, handle: u16) -> Result<(), String> {
        if self.open_files.remove(&handle).is_none() {
            Err(format!("Invalid file handle {}", handle))
        } else {
            Ok(())
        }
    }

    pub fn read_file(&mut self, handle: u16, buffer: &mut [u8]) -> Result<u16, String> {
        let file = self
            .open_files
            .get_mut(&handle)
            .ok_or_else(|| format!("Invalid file handle {}", handle))?;

        let bytes_read = file
            .read(buffer)
            .map_err(|e| format!("Read error on handle {}: {}", handle, e))?;

        Ok(bytes_read as u16)
    }

    pub fn write_file(&mut self, handle: u16, buffer: &[u8]) -> Result<u16, String> {
        let file = self
            .open_files
            .get_mut(&handle)
            .ok_or_else(|| format!("Invalid file handle {}", handle))?;

        let bytes_written = file
            .write(buffer)
            .map_err(|e| format!("Write error on handle {}: {}", handle, e))?;

        Ok(bytes_written as u16)
    }

    pub fn seek_file(&mut self, handle: u16, offset: i32, origin: u8) -> Result<u32, String> {
        let file = self
            .open_files
            .get_mut(&handle)
            .ok_or_else(|| format!("Invalid file handle {}", handle))?;

        let whence = match origin {
            0 => SeekFrom::Start(offset as u64),
            1 => SeekFrom::Current(offset as i64),
            2 => SeekFrom::End(offset as i64),
            _ => return Err(format!("Invalid seek origin {}", origin)),
        };

        let new_pos = file
            .seek(whence)
            .map_err(|e| format!("Seek error on handle {}: {}", handle, e))?;

        Ok(new_pos as u32)
    }
}
