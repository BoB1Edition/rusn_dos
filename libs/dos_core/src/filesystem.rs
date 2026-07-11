// Ver: 3 File: ./libs/dos_core/src/filesystem.rs
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

#[derive(Debug, Clone)]
pub struct FoundFile {
    pub name_83: String,
    pub attr: u8,
    pub time: u16,
    pub date: u16,
    pub size: u32,
}

#[derive(Debug)]
struct SearchContext {
    pattern: String,
    entries: Vec<fs::DirEntry>,
    index: usize,
}

#[derive(Debug)]
pub struct FileSystem {
    drivers: HashMap<char, DiskDriver>,
    current_directories: HashMap<char, String>,
    current_drive: char,
    open_files: HashMap<u16, StdFile>,
    next_handle: u16,
    search_contexts: HashMap<u32, SearchContext>, // Ключ = физический адрес DTA
}

impl FileSystem {
    pub fn new() -> Self {
        Self {
            drivers: HashMap::new(),
            open_files: HashMap::new(),
            next_handle: 5,
            current_directories: HashMap::new(),
            current_drive: 'C',
            search_contexts: HashMap::new(),
        }
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
        let normalized = path
            .trim_matches(|c| c == '\\' || c == '/')
            .replace('/', "\\");
        self.current_directories.insert(letter, normalized);
        Ok(())
    }

    pub fn get_current_directory(&self, drive_letter: char) -> Option<&str> {
        self.current_directories
            .get(&drive_letter.to_ascii_uppercase())
            .map(|s| s.as_str())
    }

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

    pub fn resolve_path(&self, dos_path: &str) -> Result<PathBuf, String> {
        let dos_path = dos_path.trim();
        if dos_path.is_empty() {
            return Err("Empty path".to_string());
        }

        // 1. Определяем букву диска и относительную часть
        let (drive_letter, relative_path) =
            if dos_path.len() >= 2 && dos_path.chars().nth(1) == Some(':') {
                let d = dos_path.chars().next().unwrap().to_ascii_uppercase();
                (d, &dos_path[2..])
            } else {
                (self.current_drive, dos_path)
            };

        let driver = self
            .drivers
            .get(&drive_letter)
            .ok_or_else(|| format!("Drive {} not configured", drive_letter))?;

        let current_dir = self
            .current_directories
            .get(&drive_letter)
            .map(|s| s.as_str())
            .unwrap_or("");
        let is_absolute = relative_path.starts_with('\\') || relative_path.starts_with('/');
        let path_str = relative_path.trim_start_matches(|c| c == '\\' || c == '/');

        let mut local_path = driver.root_path.clone();
        if !is_absolute && !current_dir.is_empty() {
            local_path.push(current_dir);
        }

        let normalized = path_str.replace('\\', "/");
        for component in normalized.split('/') {
            match component {
                "" | "." => continue,
                ".." => {
                    if local_path == driver.root_path {
                        return Err(format!("Path traversal blocked: {}", dos_path));
                    }
                    local_path.pop();
                }
                _ => local_path.push(component),
            }
        }

        // Безопасность: проверяем, что путь не вышел за пределы root
        if let (Ok(canonical_root), Ok(canonical_path)) =
            (driver.root_path.canonicalize(), local_path.canonicalize())
        {
            if !canonical_path.starts_with(&canonical_root) {
                return Err(format!("Path traversal blocked: {}", dos_path));
            }
        }

        Ok(local_path)
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

        // Определяем букву диска для проверки read_only
        let drive_letter = if dos_path.len() >= 2 && dos_path.chars().nth(1) == Some(':') {
            dos_path.chars().next().unwrap().to_ascii_uppercase()
        } else {
            self.current_drive
        };

        if (access_mode == 1 || access_mode == 2) && self.is_read_only(drive_letter) {
            return Err(format!("Attempt to write to read-only drive: {}", dos_path));
        }

        let file = match access_mode {
            0 => StdFile::open(&local_path),
            1 => OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&local_path),
            2 => OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&local_path),
            _ => Err(std::io::Error::from(std::io::ErrorKind::InvalidInput)),
        }
        .map_err(|e| format!("Failed to open file '{}': {}", local_path.display(), e))?;

        let handle = self.next_handle;
        self.next_handle = self.next_handle.wrapping_add(1);
        if self.next_handle < 5 {
            self.next_handle = 5;
        }
        self.open_files.insert(handle, file);
        Ok(handle)
    }

    pub fn create_file(&mut self, dos_path: &str, _attributes: u16) -> Result<u16, String> {
        let local_path = self.resolve_path(dos_path)?;
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&local_path)
            .map_err(|e| format!("Failed to create file '{}': {}", local_path.display(), e))?;

        let handle = self.next_handle;
        self.next_handle = self.next_handle.wrapping_add(1);
        if self.next_handle < 5 {
            self.next_handle = 5;
        }
        self.open_files.insert(handle, file);
        Ok(handle)
    }

    pub fn delete_file(&mut self, dos_path: &str) -> Result<(), String> {
        let local_path = self.resolve_path(dos_path)?;
        if local_path.is_dir() {
            return Err("Cannot delete directory with delete_file".to_string());
        }
        std::fs::remove_file(&local_path)
            .map_err(|e| format!("Failed to delete '{}': {}", local_path.display(), e))
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
            .map_err(|e| format!("Read error: {}", e))?;
        Ok(bytes_read as u16)
    }

    pub fn write_file(&mut self, handle: u16, buffer: &[u8]) -> Result<u16, String> {
        let file = self
            .open_files
            .get_mut(&handle)
            .ok_or_else(|| format!("Invalid file handle {}", handle))?;
        let bytes_written = file
            .write(buffer)
            .map_err(|e| format!("Write error: {}", e))?;
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
            .map_err(|e| format!("Seek error: {}", e))?;
        Ok(new_pos as u32)
    }

    // === Поиск файлов (Find First / Find Next) ===

    pub fn find_first(
        &mut self,
        dos_path: &str,
        dta_addr: u32,
    ) -> Result<Option<FoundFile>, String> {
        let (drive_letter, path_part) =
            if dos_path.len() >= 2 && dos_path.chars().nth(1) == Some(':') {
                (
                    dos_path.chars().next().unwrap().to_ascii_uppercase(),
                    &dos_path[2..],
                )
            } else {
                (self.current_drive, dos_path)
            };

        let driver = self.drivers.get(&drive_letter).ok_or("Drive not found")?;
        let current_dir = self
            .current_directories
            .get(&drive_letter)
            .map(|s| s.as_str())
            .unwrap_or("");
        let is_absolute = path_part.starts_with('\\') || path_part.starts_with('/');
        let path_str = path_part.trim_start_matches(|c| c == '\\' || c == '/');

        let mut dir_path = driver.root_path.clone();
        if !is_absolute && !current_dir.is_empty() {
            dir_path.push(current_dir);
        }

        let normalized = path_str.replace('\\', "/");
        let parts: Vec<&str> = normalized.split('/').collect();
        let pattern = parts.last().unwrap_or(&"*").to_string();

        for &comp in &parts[..parts.len().saturating_sub(1)] {
            match comp {
                "" | "." => continue,
                ".." => {
                    dir_path.pop();
                }
                _ => dir_path.push(comp),
            }
        }

        if !dir_path.exists() || !dir_path.is_dir() {
            self.search_contexts.remove(&dta_addr);
            return Ok(None);
        }

        let mut entries = Vec::new();
        if let Ok(read_dir) = fs::read_dir(&dir_path) {
            for entry in read_dir.flatten() {
                let name = entry.file_name().to_string_lossy().to_uppercase();
                if match_pattern(&name, &pattern.to_uppercase()) {
                    entries.push(entry);
                }
            }
        }

        if entries.is_empty() {
            self.search_contexts.remove(&dta_addr);
            return Ok(None);
        }

        self.search_contexts.insert(
            dta_addr,
            SearchContext {
                pattern,
                entries,
                index: 0,
            },
        );

        self.get_found_file(dta_addr, 0)
    }

    pub fn find_next(&mut self, dta_addr: u32) -> Result<Option<FoundFile>, String> {
        let next_index = if let Some(ctx) = self.search_contexts.get_mut(&dta_addr) {
            ctx.index += 1;
            if ctx.index >= ctx.entries.len() {
                return Ok(None);
            }
            ctx.index // Возвращаем значение из блока
        } else {
            return Ok(None);
        };
        self.get_found_file(dta_addr, next_index)
    }

    fn get_found_file(&self, dta_addr: u32, index: usize) -> Result<Option<FoundFile>, String> {
        let ctx = self.search_contexts.get(&dta_addr).ok_or("Context lost")?;
        if index >= ctx.entries.len() {
            return Ok(None);
        }
        let entry = &ctx.entries[index];
        let metadata = entry.metadata().map_err(|e| e.to_string())?;

        let name = entry.file_name().to_string_lossy().to_uppercase();
        let name_83 = format_83(&name);

        // Упрощенное получение времени/даты (в реальном DOS это из metadata)
        let time = 0;
        let date = 0;

        Ok(Some(FoundFile {
            name_83,
            attr: if metadata.is_dir() { 0x10 } else { 0x20 }, // 0x10 = Directory, 0x20 = Archive
            time,
            date,
            size: metadata.len() as u32,
        }))
    }
}

// === Вспомогательные функции ===

fn match_pattern(name: &str, pattern: &str) -> bool {
    let n: Vec<char> = name.chars().collect();
    let p: Vec<char> = pattern.chars().collect();

    fn matches(n: &[char], p: &[char]) -> bool {
        if p.is_empty() {
            return n.is_empty();
        }
        if p[0] == '*' {
            for i in 0..=n.len() {
                if matches(&n[i..], &p[1..]) {
                    return true;
                }
            }
            return false;
        } else if p[0] == '?' {
            if n.is_empty() {
                return false;
            }
            return matches(&n[1..], &p[1..]);
        } else {
            if n.is_empty() {
                return false;
            }
            if n[0].to_ascii_uppercase() == p[0].to_ascii_uppercase() {
                return matches(&n[1..], &p[1..]);
            }
            return false;
        }
    }
    matches(&n, &p)
}

fn format_83(name: &str) -> String {
    if let Some(dot_pos) = name.find('.') {
        let n = &name[..dot_pos];
        let e = &name[dot_pos + 1..];
        format!("{:<8}.{: <3}", n, e).trim().to_string()
    } else {
        format!("{:<8}", name).trim().to_string()
    }
}
