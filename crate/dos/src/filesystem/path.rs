// Ver: 1 File: crate/dos/src/filesystem/path.rs

use std::path::PathBuf;

use super::FileSystem;
use super::types::DeviceKind;

/// Преобразует DOS-путь в путь хоста с учётом текущего каталога диска.
pub(super) fn resolve_path(fs: &FileSystem, dos_path: &str) -> Result<PathBuf, String> {
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
            (fs.current_drive, dos_path)
        };
    let driver = fs
        .drivers
        .get(&drive_letter)
        .ok_or_else(|| format!("Drive {} not configured", drive_letter))?;
    let current_dir = fs
        .current_directories
        .get(&drive_letter)
        .map(|s| s.as_str())
        .unwrap_or("");
    let is_absolute = relative_path.starts_with('\\') || relative_path.starts_with('/');
    let path_str = relative_path.trim_start_matches(['\\', '/']);
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

/// Разрешает имя DOS-устройства (CON, NUL, PRN, AUX, COMx, LPTx, CLOCK$, EMMXXXX0).
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
    let name = path.rsplit(['\\', '/']).next().unwrap_or("");
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
