// Ver: 5 File: crate/dos/src/filesystem.rs
use bus::Machine;
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

    pub fn device_of(&self, handle: u16) -> Option<DeviceKind> {
        match self.open_files.get(&handle) {
            Some(FileHandle::Device(kind)) => Some(*kind),
            _ => None,
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

        Ok(local_path)
    }

    pub fn is_read_only(&self, drive_letter: char) -> bool {
        self.drivers
            .get(&drive_letter.to_ascii_uppercase())
            .map_or(true, |d| d.read_only)
    }

    pub fn open_file(&mut self, dos_path: &str, access_mode: u8) -> Result<u16, String> {
        if let Some(device) = resolve_device(dos_path) {
            let handle = self.next_handle;
            self.next_handle = self.next_handle.wrapping_add(1);
            if self.next_handle < 5 {
                self.next_handle = 5;
            }
            self.open_files.insert(handle, FileHandle::Device(device));
            log::info!("Opened virtual device: {:?}", device);
            return Ok(handle);
        }
        let local_path = self.resolve_for_open(dos_path, access_mode)?;
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
        log::info!("Opened file: {:?}:{:?}", local_path, file);
        self.handle_paths.insert(handle, local_path);
        self.open_files.insert(handle, FileHandle::Disk(file));
        Ok(handle)
    }

    pub fn create_file(&mut self, dos_path: &str, _attributes: u16) -> Result<u16, String> {
        if let Some(device) = resolve_device(dos_path) {
            let handle = self.next_handle;
            self.next_handle = self.next_handle.wrapping_add(1);
            if self.next_handle < 5 {
                self.next_handle = 5;
            }
            self.open_files.insert(handle, FileHandle::Device(device));
            log::info!("Created virtual device: {:?}", device);
            return Ok(handle);
        }
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
        log::info!("Created file: {:?}:{:?}", local_path, file);
        self.open_files.insert(handle, FileHandle::Disk(file));
        self.handle_paths.insert(handle, local_path);
        Ok(handle)
    }

    pub fn delete_file(&mut self, dos_path: &str) -> Result<(), String> {
        if resolve_device(dos_path).is_some() {
            return Err("Access denied: Cannot delete a device".to_string());
        }

        let local_path = self.resolve_path(dos_path)?;
        if local_path.is_dir() {
            return Err("Cannot delete directory with delete_file".to_string());
        }
        std::fs::remove_file(&local_path)
            .map_err(|e| format!("Failed to delete '{}': {}", local_path.display(), e))
    }

    pub fn close_file(&mut self, handle: u16) -> Result<(), String> {
        self.handle_paths.remove(&handle);
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

        match file {
            FileHandle::Disk(f) => {
                let bytes_read = f.read(buffer).map_err(|e| format!("Read error: {}", e))?;
                Ok(bytes_read as u16)
            }
            FileHandle::Device(DeviceKind::Con) => Ok(0),
            FileHandle::Device(_) => {
                // NUL, PRN, COM, CLOCK, EMS при чтении всегда возвращают EOF (0 байт)
                Ok(0)
            }
        }
    }

    pub fn write_file(&mut self, handle: u16, buffer: &[u8]) -> Result<u16, String> {
        let file = self
            .open_files
            .get_mut(&handle)
            .ok_or_else(|| format!("Invalid file handle {}", handle))?;

        match file {
            FileHandle::Disk(f) => {
                let bytes_written = f.write(buffer).map_err(|e| format!("Write error: {}", e))?;
                Ok(bytes_written as u16)
            }
            FileHandle::Device(DeviceKind::Con) => {
                let bytes_written = std::io::stdout()
                    .write(buffer)
                    .map_err(|e| format!("CON write error: {}", e))?;
                std::io::stdout().flush().ok();
                Ok(bytes_written as u16)
            }
            FileHandle::Device(DeviceKind::Nul) => {
                Ok(buffer.len() as u16) // NUL просто "съедает" данные
            }
            FileHandle::Device(DeviceKind::Lpt(_)) => {
                log::info!("[PRN/LPT OUTPUT] {}", String::from_utf8_lossy(buffer));
                Ok(buffer.len() as u16)
            }
            FileHandle::Device(DeviceKind::Com(_)) => {
                log::info!("[AUX/COM OUTPUT] {}", String::from_utf8_lossy(buffer));
                Ok(buffer.len() as u16)
            }
            FileHandle::Device(DeviceKind::Clock) | FileHandle::Device(DeviceKind::Ems) => {
                log::debug!("Write to CLOCK$/EMMXXXX0 ignored");
                Ok(buffer.len() as u16)
            }
        }
    }

    pub fn seek_file(&mut self, handle: u16, offset: i32, origin: u8) -> Result<u32, String> {
        let file = self
            .open_files
            .get_mut(&handle)
            .ok_or_else(|| format!("Invalid file handle {}", handle))?;

        match file {
            FileHandle::Disk(f) => {
                let whence = match origin {
                    0 => SeekFrom::Start(offset as u64),
                    1 => SeekFrom::Current(offset as i64),
                    2 => SeekFrom::End(offset as i64),
                    _ => return Err(format!("Invalid seek origin {}", origin)),
                };
                let new_pos = f.seek(whence).map_err(|e| format!("Seek error: {}", e))?;
                Ok(new_pos as u32)
            }
            FileHandle::Device(_) => {
                Ok(0) // Устройства не поддерживают позиционирование
            }
        }
    }

    pub fn find_first(
        &mut self,
        dos_path: &str,
        dta_addr: u32,
    ) -> Result<Option<FoundFile>, String> {
        // --- Шаг 1: диск, путь, маска (как раньше) ---
        let (drive_letter, path_part) =
            if dos_path.len() >= 2 && dos_path.chars().nth(1) == Some(':') {
                (
                    dos_path.chars().next().unwrap().to_ascii_uppercase(),
                    &dos_path[2..],
                )
            } else {
                (self.current_drive, dos_path)
            };
        let is_absolute = path_part.starts_with('\\') || path_part.starts_with('/');
        let path_str = path_part.trim_start_matches(|c| c == '\\' || c == '/');
        let normalized = path_str.replace('\\', "/");
        let parts: Vec<&str> = normalized.split('/').collect();
        let pattern = parts.last().unwrap_or(&"*").to_string();
        let dir_comps: Vec<String> = parts[..parts.len().saturating_sub(1)]
            .iter()
            .map(|s| s.to_string())
            .collect();

        // --- Шаг 2: «голая» маска -> перебор каталогов поиска, иначе один каталог ---
        let bare = !dos_path.contains('\\')
            && !dos_path.contains('/')
            && !(dos_path.len() >= 2 && dos_path.chars().nth(1) == Some(':'));

        // (диск, компоненты каталога, признак «из search_candidates»)
        let candidates: Vec<(char, Vec<String>, bool)> = if bare {
            self.search_candidates()
                .into_iter()
                .map(|(d, dir)| {
                    let comps = dir
                        .split('\\')
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .collect();
                    (d, comps, true)
                })
                .collect()
        } else {
            let c = vec![(drive_letter, dir_comps, false)];
            c
        };

        // --- Шаг 3: первый каталог с совпадениями побеждает ---
        for (drv, comps, from_search) in candidates {
            let driver = match self.drivers.get(&drv) {
                Some(d) => d,
                None => continue,
            };
            let mut dir_path = driver.root_path.clone();
            if !from_search {
                // старое поведение для явных/относительных путей
                let current_dir = self
                    .current_directories
                    .get(&drv)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                if !is_absolute && !current_dir.is_empty() {
                    dir_path.push(current_dir);
                }
            }
            for comp in &comps {
                match comp.as_str() {
                    "" | "." => continue,
                    ".." => {
                        dir_path.pop();
                    }
                    _ => dir_path.push(comp),
                }
            }
            if !dir_path.exists() || !dir_path.is_dir() {
                continue; // каталога нет — пробуем следующего кандидата
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
                continue; // в этом каталоге совпадений нет — пробуем дальше
            }
            self.search_contexts.insert(
                dta_addr,
                SearchContext {
                    pattern: pattern.clone(),
                    entries,
                    index: 0,
                },
            );
            return self.get_found_file(dta_addr, 0);
        }
        self.search_contexts.remove(&dta_addr);
        Ok(None)
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
        let (date, time) = metadata
            .modified()
            .ok()
            .map(system_time_to_dos_datetime)
            .unwrap_or((0, 0));

        Ok(Some(FoundFile {
            name_83,
            attr: if metadata.is_dir() { 0x10 } else { 0x20 }, // 0x10 = Directory, 0x20 = Archive
            time,
            date,
            size: metadata.len() as u32,
        }))
    }
    pub fn handle_path(&self, handle: u16) -> Option<&PathBuf> {
        self.handle_paths.get(&handle)
    }

    pub fn set_program_location(&mut self, drive: char, dir: &str) {
        self.program_drive = drive.to_ascii_uppercase();
        self.program_dir = dir.trim_matches(|c| c == '\\' || c == '/').to_string();
    }
    pub fn set_search_path_from_env(&mut self, path_value: &str) {
        self.search_path = path_value
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
            .collect();
    }
    fn search_candidates(&self) -> Vec<(char, String)> {
        let mut v = vec![(
            self.current_drive,
            self.current_directories
                .get(&self.current_drive)
                .cloned()
                .unwrap_or_default(),
        )];
        if !self.program_dir.is_empty() {
            v.push((self.program_drive, self.program_dir.clone()));
        }
        v.extend(self.search_path.iter().cloned());
        v
    }
    fn is_bare_name(&self, p: &str) -> bool {
        !(p.len() >= 2 && p.chars().nth(1) == Some(':'))
            && !p.starts_with('\\')
            && !p.starts_with('/')
    }

    pub fn resolve_for_open(&self, dos_path: &str, mode: u8) -> Result<PathBuf, String> {
        if !self.is_bare_name(dos_path) || mode == 1 {
            return self.resolve_path(dos_path);
        }
        for (drive, dir) in self.search_candidates() {
            let full = if dir.is_empty() {
                &format!("{}:\\{}", drive, dos_path)
            } else {
                &format!("{}:\\{}\\{}", drive, dir, dos_path)
            };
            if let Ok(p) = self.resolve_path(&full) {
                if p.is_file() {
                    return Ok(p);
                }
            }
        }
        if mode == 2 {
            return self.resolve_path(dos_path); // создавать — в cwd
        }
        Err(format!("File not found: {}", dos_path))
    }
}

impl Default for FileSystem {
    fn default() -> Self {
        Self::new()
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Con,
    Nul,
    Com(u8),
    Lpt(u8),
    Clock,
    Ems,
}

#[derive(Debug)]
pub enum FileHandle {
    Disk(StdFile),
    Device(DeviceKind),
}

pub fn resolve_device(dos_path: &str) -> Option<DeviceKind> {
    // 1. Отбрасываем префикс диска: "C:\DIR\CON.TXT" -> "\DIR\CON.TXT"
    let path = if dos_path.len() >= 2
        && dos_path.as_bytes()[1] == b':'
        && dos_path.as_bytes()[0].is_ascii_alphabetic()
    {
        &dos_path[2..]
    } else {
        dos_path
    };
    let name = path.rsplit(|c| c == '\\' || c == '/').next().unwrap_or("");
    let raw = name.split('.').next().unwrap_or("");
    let base = raw.trim_end().to_ascii_uppercase();
    let base = base.trim_end_matches(':');

    match base {
        "CON" => Some(DeviceKind::Con),
        "NUL" => Some(DeviceKind::Nul),
        "CLOCK$" => Some(DeviceKind::Clock),
        "EMMXXXX0" => Some(DeviceKind::Ems),
        "PRN" => Some(DeviceKind::Lpt(1)),
        "AUX" => Some(DeviceKind::Com(1)),
        _ => {
            if let Some(n) = base.strip_prefix("LPT") {
                parse_port(n).map(DeviceKind::Lpt)
            } else if let Some(n) = base.strip_prefix("COM") {
                parse_port(n).map(DeviceKind::Com)
            } else {
                None
            }
        }
    }
}

fn parse_port(s: &str) -> Option<u8> {
    if s.is_empty() {
        return None;
    }
    let n: u8 = s.parse().ok()?;
    if (1..=4).contains(&n) { Some(n) } else { None }
}

pub fn system_time_to_dos_datetime(t: std::time::SystemTime) -> (u16, u16) {
    let dur = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();
    let days = secs / 86400;
    let sod = secs % 86400;

    let mut year: u64 = 1970;
    let mut rem = days;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let diy: u64 = if leap { 366 } else { 365 };
        if rem < diy {
            break;
        }
        rem -= diy;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let dim: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month: u64 = 1;
    for &d in dim.iter() {
        if rem < d {
            break;
        }
        rem -= d;
        month += 1;
    }
    let day = rem + 1;

    // DOS хранит год как (год-1980) в 7 битах: диапазон 1980..=2107
    let year = year.clamp(1980, 2107);

    let hours = sod / 3600;
    let minutes = (sod % 3600) / 60;
    let seconds = sod % 60;

    let date = (((year - 1980) as u16) << 9) | ((month as u16) << 5) | (day as u16);
    let time = ((hours as u16) << 11) | ((minutes as u16) << 5) | ((seconds / 2) as u16);
    (date, time)
}
