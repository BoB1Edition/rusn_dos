// Ver: 1 File: crate/dos/src/filesystem/types.rs

use std::path::PathBuf;

/// Смонтированный диск хоста, представленный DOS-буквой.
#[derive(Debug, Clone)]
pub struct DiskDriver {
    pub letter: char,
    pub root_path: PathBuf,
    pub read_only: bool,
}

/// Найденный по шаблону DTA-файл (INT 21h/AH=4Eh,4Fh).
#[derive(Debug, Clone)]
pub struct FoundFile {
    pub name_83: String,
    pub attr: u8,
    pub time: u16,
    pub date: u16,
    pub size: u32,
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
    Disk(std::fs::File),
    Device(DeviceKind),
}

/// Контекст активного поиска по шаблону (DTA) для INT 21h/AH=4Eh,4Fh.
#[derive(Debug)]
pub struct SearchContext {
    pub entries: Vec<std::fs::DirEntry>,
    pub index: usize,
}
