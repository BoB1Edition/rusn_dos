// Ver: 1 File: crate/dos/src/bat/runner/util.rs

use std::io::Write;
use std::path::{Path, PathBuf};

use super::types::ShellState;

pub(crate) fn shell_print(s: &str) {
    println!("{}", s);
    std::io::stdout().flush().ok();
}

pub(crate) fn parse_code(rest: &str) -> Option<u8> {
    rest.trim().parse::<u8>().ok()
}

pub(crate) fn split_first(line: &str) -> (&str, &str) {
    let mut it = line.trim().splitn(2, char::is_whitespace);
    let a = it.next().unwrap_or("");
    let b = it.next().unwrap_or("").trim();
    (a, b)
}

/// Токены с уважением к кавычкам (кавычки сохраняются внутри токена, как в DOS).
pub(crate) fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    for c in s.chars() {
        match c {
            '"' => {
                in_q = !in_q;
                cur.push('"');
            }
            c if c.is_whitespace() && !in_q => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Раскрытие %0..%9 и %VAR% (неизвестная переменная -> пустая строка, %% -> %).
pub(crate) fn expand(line: &str, state: &ShellState, params: &[String]) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] != '%' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        if i + 1 >= chars.len() {
            out.push('%');
            break;
        }
        let c = chars[i + 1];
        if c == '%' {
            out.push('%');
            i += 2;
            continue;
        }
        if c.is_ascii_digit() {
            let idx = (c as u8 - b'0') as usize;
            if let Some(p) = params.get(idx) {
                out.push_str(p);
            }
            i += 2;
            continue;
        }
        if let Some(close) = chars[i + 1..].iter().position(|&ch| ch == '%') {
            let name: String = chars[i + 1..i + 1 + close].iter().collect();
            let val = state
                .env
                .get(&name.to_ascii_uppercase())
                .cloned()
                .unwrap_or_default();
            out.push_str(&val);
            i += 2 + close;
            continue;
        }
        out.push('%');
        i += 1;
    }
    out
}

pub(crate) fn split_dir_name(p: &str) -> (String, String) {
    match p.rsplit_once(['\\', '/']) {
        Some((d, n)) => (d.to_string(), n.to_string()),
        None => (String::new(), p.to_string()),
    }
}

pub(crate) fn join_dos(base: &str, extra: &str) -> String {
    let mut v: Vec<&str> = Vec::new();
    for part in [base, extra] {
        for c in part.split(['\\', '/']) {
            if !c.is_empty() {
                v.push(c);
            }
        }
    }
    v.join("\\")
}

/// Регистронезависимый поиск файла в каталоге хоста.
pub(crate) fn find_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    let direct = dir.join(name);
    if direct.is_file() {
        return Some(direct);
    }
    let rd = std::fs::read_dir(dir).ok()?;
    for ent in rd.flatten() {
        if ent.file_name().to_string_lossy().eq_ignore_ascii_case(name) && ent.path().is_file() {
            return Some(ent.path());
        }
    }
    None
}
