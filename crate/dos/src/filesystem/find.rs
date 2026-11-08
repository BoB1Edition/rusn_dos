// Ver: 1 File: crate/dos/src/filesystem/find.rs

use std::fs;

use super::FileSystem;
use super::types::{FoundFile, SearchContext};

impl FileSystem {
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
        let path_str = path_part.trim_start_matches(['\\', '/']);
        let normalized = path_str.replace('\\', "/");
        let parts: Vec<&str> = normalized.split('/').collect();
        let pattern = parts.last().unwrap_or(&"*").to_string();
        let dir_comps: Vec<String> = parts[..parts.len().saturating_sub(1)]
            .iter()
            .map(|s| s.to_string())
            .collect();

        // --- Шаг 2: «голая» маска -> перебор каталогов поиска, иначе один каталог ---
        let bare = !(dos_path.contains('\\')
            || dos_path.contains('/')
            || (dos_path.len() >= 2 && dos_path.chars().nth(1) == Some(':')));

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
            self.search_contexts
                .insert(dta_addr, SearchContext { entries, index: 0 });
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
            false
        } else if p[0] == '?' {
            if n.is_empty() {
                false
            } else {
                matches(&n[1..], &p[1..])
            }
        } else if n.is_empty() {
            false
        } else if n[0].eq_ignore_ascii_case(&p[0]) {
            matches(&n[1..], &p[1..])
        } else {
            false
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

/// Преобразует время файловой системы хоста в пару (дата DOS, время DOS).
pub fn system_time_to_dos_datetime(t: std::time::SystemTime) -> (u16, u16) {
    let dur = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();
    let days = secs / 86400;
    let sod = secs % 86400;

    let mut year: u64 = 1970;
    let mut rem = days;
    loop {
        let leap = is_leap(year);
        let diy: u64 = if leap { 366 } else { 365 };
        if rem < diy {
            break;
        }
        rem -= diy;
        year += 1;
    }
    let leap = is_leap(year);
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

fn is_leap(year: u64) -> bool {
    year.is_multiple_of(4) && !year.is_multiple_of(100) || year.is_multiple_of(400)
}
