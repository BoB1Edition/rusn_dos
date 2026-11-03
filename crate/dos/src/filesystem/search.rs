// Ver: 1 File: crate/dos/src/filesystem/search.rs

use super::path::resolve_path;

impl super::FileSystem {
    /// Запоминает каталог, где лежит исполняемый файл (для поиска «голых» имён).
    pub fn set_program_location(&mut self, drive: char, dir: &str) {
        self.program_drive = drive.to_ascii_uppercase();
        self.program_dir = dir.trim_matches(['\\', '/']).to_string();
    }

    /// Разбирает PATH из окружения DOS в список (диск, каталог).
    pub fn set_search_path_from_env(&mut self, path_value: &str) {
        self.search_path = path_value
            .split(';')
            .filter_map(|e| {
                let e = e.trim();
                if e.len() >= 2 && e.chars().nth(1) == Some(':') {
                    Some((
                        e.chars().next().unwrap().to_ascii_uppercase(),
                        e[2..].trim_matches(['\\', '/']).to_string(),
                    ))
                } else {
                    None
                }
            })
            .collect();
    }

    /// Каталоги поиска для «голых» имён: CWD → каталог программы → PATH.
    pub(super) fn search_candidates(&self) -> Vec<(char, String)> {
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
        !(p.starts_with('\\')
            || p.starts_with('/')
            || (p.len() >= 2 && p.chars().nth(1) == Some(':')))
    }

    /// Разрешает путь при INT 21h/AH=3Dh: для «голых» имён ищет по каталогам поиска.
    pub fn resolve_for_open(&self, dos_path: &str, mode: u8) -> Result<std::path::PathBuf, String> {
        if !self.is_bare_name(dos_path) || mode == 1 {
            return resolve_path(self, dos_path);
        }
        for (drive, dir) in self.search_candidates() {
            let full = if dir.is_empty() {
                format!("{}:\\{}", drive, dos_path)
            } else {
                format!("{}:\\{}\\{}", drive, dir, dos_path)
            };
            if let Ok(p) = resolve_path(self, &full)
                && p.is_file()
            {
                return Ok(p);
            }
        }
        if mode == 2 {
            return resolve_path(self, dos_path); // создавать — в cwd
        }
        Err(format!("File not found: {}", dos_path))
    }
}
