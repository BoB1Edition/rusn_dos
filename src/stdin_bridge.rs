// Ver: 1 File: ./src/stdin_bridge.rs
//! Мост между терминалом хоста (stdin) и эмулируемой клавиатурой.
//! Используется только в консольном режиме (без -g).
use bus::peripherals::keyboard::Keyboard;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal;
use std::time::Duration;


pub const NO_STDIN_KEYS_ERR: &str = "--no_stdin_keys: программа ожидает ввод с клавиатуры; не используйте этот флаг с интерактивными приложениями";

/// RAII-гард «сырого режима» терминала: восстанавливает терминал
/// при любом выходе (return, ошибка, паника).
pub struct RawTerminal;

impl RawTerminal {
    /// Пытается включить raw mode. Если stdin — не tty (редирект/пайп),
    /// возвращает None: ввод продолжит работать, но построчно.
    pub fn try_enable() -> Option<Self> {
        match terminal::enable_raw_mode() {
            Ok(()) => {
                log::info!("Console raw mode enabled (stdin bridge active)");
                Some(Self)
            }
            Err(e) => {
                log::warn!(
                    "Cannot enable raw mode ({}). Input will be line-buffered.",
                    e
                );
                None
            }
        }
    }
}

impl Drop for RawTerminal {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        log::debug!("Console raw mode disabled");
    }
}

/// Неблокирующий опрос stdin: все накопленные нажатия уходят в буфер клавиатуры.
pub fn poll_keys(kb: &mut Keyboard) {
    loop {
        match event::poll(Duration::ZERO) {
            Ok(true) => {}
            _ => break,
        }
        match event::read() {
            Ok(Event::Key(key)) => {
                if let Some((sc, ascii)) = map_key(key) {
                    kb.push_key(sc, ascii);
                }
            }
            Ok(_) => {}      // ✅ ИСПРАВЛЕНО: Ok(_) вместо Ok()
            Err(_) => break, // ✅ ИСПРАВЛЕНО: Err(_) вместо Err()
        }
    }
}

/// Преобразование KeyEvent в пару (scancode, ascii) для DOS/BIOS.
fn map_key(key: KeyEvent) -> Option<(u8, u8)> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Enter => Some((0x1C, 0x0D)),
        KeyCode::Backspace => Some((0x0E, 0x08)),
        KeyCode::Tab => Some((0x0F, 0x09)),
        KeyCode::Esc => Some((0x01, 0x1B)),
        KeyCode::Up => Some((0x48, 0)),
        KeyCode::Down => Some((0x50, 0)),
        KeyCode::Left => Some((0x4B, 0)),
        KeyCode::Right => Some((0x4D, 0)),
        KeyCode::Home => Some((0x47, 0)),
        KeyCode::End => Some((0x4F, 0)),
        KeyCode::PageUp => Some((0x49, 0)),
        KeyCode::PageDown => Some((0x51, 0)),
        KeyCode::Insert => Some((0x52, 0)),
        KeyCode::Delete => Some((0x53, 0)),
        KeyCode::F(n) if (1..=10).contains(&n) => Some((0x3B + n - 1, 0)),
        KeyCode::Char(c) => {
            let ascii = if ctrl {
                match c.to_ascii_uppercase() {
                    'A'..='Z' => (c.to_ascii_uppercase() as u8) - b'A' + 1,
                    _ => c as u8,
                }
            } else {
                c as u8
            };
            Some((scancode_for_char(c), ascii))
        }
        // Одинокие модификаторы (Shift/Ctrl/Alt) в буфер не попадают —
        // как и в реальном BIOS (они меняют только shift-флаги).
        _ => None,
    }
}

/// Scancode US-раскладки для символа (с учётом shift-символов).
fn scancode_for_char(c: char) -> u8 {
    match c.to_ascii_lowercase() {
        'a' => 0x1E,
        'b' => 0x30,
        'c' => 0x2E,
        'd' => 0x20,
        'e' => 0x12,
        'f' => 0x21,
        'g' => 0x22,
        'h' => 0x23,
        'i' => 0x17,
        'j' => 0x24,
        'k' => 0x25,
        'l' => 0x26,
        'm' => 0x32,
        'n' => 0x31,
        'o' => 0x18,
        'p' => 0x19,
        'q' => 0x10,
        'r' => 0x13,
        's' => 0x1F,
        't' => 0x14,
        'u' => 0x16,
        'v' => 0x2F,
        'w' => 0x11,
        'x' => 0x2D,
        'y' => 0x15,
        'z' => 0x2C,
        '1' => 0x02,
        '2' => 0x03,
        '3' => 0x04,
        '4' => 0x05,
        '5' => 0x06,
        '6' => 0x07,
        '7' => 0x08,
        '8' => 0x09,
        '9' => 0x0A,
        '0' => 0x0B,
        ' ' => 0x39,
        '-' => 0x0C,
        '=' => 0x0D,
        '[' => 0x1A,
        ']' => 0x1B,
        ';' => 0x27,
        '\'' => 0x28,
        '`' => 0x29,
        '\\' => 0x2B,
        ',' => 0x33,
        '.' => 0x34,
        '/' => 0x35,
        '!' => 0x02,
        '@' => 0x03,
        '#' => 0x04,
        '$' => 0x05,
        '%' => 0x06,
        '^' => 0x07,
        '&' => 0x08,
        '*' => 0x09,
        '(' => 0x0A,
        ')' => 0x0B,
        '_' => 0x0C,
        '+' => 0x0D,
        '{' => 0x1A,
        '}' => 0x1B,
        ':' => 0x27,
        '"' => 0x28,
        '~' => 0x29,
        '|' => 0x2B,
        '<' => 0x33,
        '>' => 0x34,
        '?' => 0x35,
        '\n' | '\r' => 0x1C,
        _ => 0x00,
    }
}
