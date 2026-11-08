// Ver: 1 File: crate/dos/src/bat/runner/resolve.rs

use std::path::PathBuf;

use super::util::{find_ci, join_dos, split_dir_name};

impl super::Runner {
    pub(crate) fn path_entries(&self) -> Vec<(char, String)> {
        self.state
            .env
            .get("PATH")
            .map(|s| s.as_str())
            .unwrap_or("")
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
            .collect()
    }

    pub(crate) fn host_parent_dos(&self, path: &std::path::Path) -> Option<(char, String)> {
        for d in &self.drivers {
            let root = PathBuf::from(&d.root_path);
            let (p, r) = match (path.canonicalize(), root.canonicalize()) {
                (Ok(cp), Ok(cr)) => (cp, cr),
                _ => (path.to_path_buf(), root.clone()),
            };
            if let Ok(rel) = p.strip_prefix(&r) {
                let parent = rel.parent()?;
                let dos = parent
                    .to_str()?
                    .replace('/', "\\")
                    .trim_matches(['\\', '/'])
                    .to_string();
                return Some((d.letter, dos));
            }
        }
        None
    }

    pub(crate) fn resolve_command(&self, token: &str) -> Option<(PathBuf, bool)> {
        let clean = token.trim_matches('"');
        if clean.is_empty() {
            return None;
        }
        // 1) отделяем префикс диска ОТ пути
        let (drive_opt, rest_path) = if clean.len() >= 2 && clean.chars().nth(1) == Some(':') {
            (
                Some(clean.chars().next().unwrap().to_ascii_uppercase()),
                &clean[2..],
            )
        } else {
            (None, clean)
        };
        let (dirpart, namepart) = split_dir_name(rest_path);
        let names: Vec<String> = if namepart.contains('.') {
            let r = vec![namepart.clone()];
            r
        } else {
            let r = vec![
                format!("{}.COM", namepart),
                format!("{}.EXE", namepart),
                format!("{}.BAT", namepart),
            ];
            r
        };
        // 2) базы поиска: диск-абсолют | \-абсолют | cwd+PATH
        let bases: Vec<(char, String)> = if let Some(d) = drive_opt {
            let r = vec![(d, String::new())];
            r
        } else if rest_path.starts_with('\\') || rest_path.starts_with('/') {
            let r = vec![(self.state.current_drive, String::new())];
            r
        } else {
            let mut v = vec![(
                self.state.current_drive,
                self.state
                    .cwd
                    .get(&self.state.current_drive)
                    .cloned()
                    .unwrap_or_default(),
            )];
            v.extend(self.path_entries());
            v
        };

        for (drive, base) in bases {
            let driver = match self.drivers.iter().find(|d| d.letter == drive) {
                Some(d) => d,
                None => continue,
            };

            let rel_dir = join_dos(&base, &dirpart);

            // 🔧 ИСПРАВЛЕНИЕ: резолвим путь к каталогу компонент за компонентом с учётом регистра
            let mut current_host = driver.root_path.clone();
            let mut valid_dir = true;
            for comp in rel_dir.split(['\\', '/']) {
                if comp.is_empty() {
                    continue;
                }
                let direct = current_host.join(comp);
                if direct.exists() {
                    current_host = direct;
                } else {
                    let rd = match std::fs::read_dir(&current_host) {
                        Ok(r) => r,
                        Err(_) => {
                            valid_dir = false;
                            break;
                        }
                    };
                    let found = match rd
                        .flatten()
                        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(comp))
                    {
                        Some(f) => f,
                        None => {
                            valid_dir = false;
                            break;
                        }
                    };
                    current_host = found.path();
                }
            }
            if !valid_dir {
                continue;
            }

            let host_dir = current_host;

            for n in &names {
                if let Some(p) = find_ci(&host_dir, n) {
                    let is_bat = p
                        .extension()
                        .map(|e| e.to_string_lossy().to_ascii_uppercase() == "BAT")
                        .unwrap_or(false);
                    return Some((p, is_bat));
                }
            }
        }
        None
    }

    pub(crate) fn shell_exists(&self, dos_path: &str) -> bool {
        let (drive, rel) = if dos_path.len() >= 2 && dos_path.chars().nth(1) == Some(':') {
            (
                dos_path.chars().next().unwrap().to_ascii_uppercase(),
                dos_path[2..].to_string(),
            )
        } else {
            (
                self.state.current_drive,
                join_dos(
                    self.state
                        .cwd
                        .get(&self.state.current_drive)
                        .map(|s| s.as_str())
                        .unwrap_or(""),
                    dos_path,
                ),
            )
        };
        let driver = match self.drivers.iter().find(|d| d.letter == drive) {
            Some(d) => d,
            None => return false,
        };
        let mut host = driver.root_path.clone();
        let rel = rel.trim_start_matches(['\\', '/']);
        if !rel.is_empty() {
            host.push(rel);
        }
        host.exists()
    }
}
