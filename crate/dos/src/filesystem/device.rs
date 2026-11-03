// Ver: 1 File: crate/dos/src/filesystem/device.rs

use std::fs::{File as StdFile, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

use super::path::resolve_device;
use super::types::{DeviceKind, FileHandle};

impl super::FileSystem {
    pub fn device_of(&self, handle: u16) -> Option<DeviceKind> {
        match self.open_files.get(&handle) {
            Some(FileHandle::Device(kind)) => Some(*kind),
            _ => None,
        }
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
                .truncate(false)
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
}
