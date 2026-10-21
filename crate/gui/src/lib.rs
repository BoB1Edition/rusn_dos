// Ver: 4 File: crate/gui/src/lib.rs

use std::collections::HashSet;

use bus::Display;
use minifb::{Key, Window, WindowOptions};

/// Окно эмулятора на базе minifb с отслеживанием новых нажатий клавиш.
pub struct EmuWindow {
    window: Window,
    prev_keys: HashSet<Key>,
    last_frame: Vec<u32>,
    last_size: (usize, usize),
    last_update: std::time::Instant,
}

impl EmuWindow {
    pub fn new(title: &str, width: usize, height: usize) -> Result<Self, minifb::Error> {
        let window = Window::new(title, width, height, WindowOptions::default())?;
        Ok(Self {
            window,
            prev_keys: HashSet::new(),
            last_frame: vec![0u32; width * height], // чёрный кадр сразу
            last_size: (width, height),
            last_update: std::time::Instant::now(),
        })
    }

    pub fn set_target_fps(&mut self, fps: usize) {
        self.window.set_target_fps(fps);
    }

    fn pump(&mut self) {
        let (w, h) = self.last_size;
        self.last_update = std::time::Instant::now();
        self.window.update_with_buffer(&self.last_frame, w, h).ok();
    }
}

impl Display for EmuWindow {
    fn is_open(&self) -> bool {
        self.window.is_open()
    }

    fn size(&self) -> (usize, usize) {
        self.window.get_size()
    }

    fn poll_keys(&mut self) -> Vec<(u8, u8)> {
        if self.last_update.elapsed() >= std::time::Duration::from_millis(50) {
            self.pump();
        }
        let current: HashSet<Key> = self.window.get_keys().into_iter().collect();
        let mut pressed = Vec::new();
        for key in current.difference(&self.prev_keys) {
            if let Some(mapping) = minifb_to_dos(*key) {
                pressed.push(mapping);
            }
        }
        self.prev_keys = current;
        pressed
    }

    fn update(&mut self, pixels: &[u32], width: usize, height: usize) {
        self.last_frame = pixels.to_vec();
        self.last_size = (width, height);
        self.last_update = std::time::Instant::now();
        self.window.update_with_buffer(pixels, width, height).ok();
    }

    fn pump_events(&mut self) {
        self.pump();
    }
}

/// Переводит клавишу minifb в пару (scancode, ascii) для DOS/BIOS.
pub fn minifb_to_dos(key: Key) -> Option<(u8, u8)> {
    match key {
        Key::A => Some((0x1E, b'a')),
        Key::B => Some((0x30, b'b')),
        Key::C => Some((0x2E, b'c')),
        Key::D => Some((0x20, b'd')),
        Key::E => Some((0x12, b'e')),
        Key::F => Some((0x21, b'f')),
        Key::G => Some((0x22, b'g')),
        Key::H => Some((0x23, b'h')),
        Key::I => Some((0x17, b'i')),
        Key::J => Some((0x24, b'j')),
        Key::K => Some((0x25, b'k')),
        Key::L => Some((0x26, b'l')),
        Key::M => Some((0x32, b'm')),
        Key::N => Some((0x31, b'n')),
        Key::O => Some((0x18, b'o')),
        Key::P => Some((0x19, b'p')),
        Key::Q => Some((0x10, b'q')),
        Key::R => Some((0x13, b'r')),
        Key::S => Some((0x1F, b's')),
        Key::T => Some((0x14, b't')),
        Key::U => Some((0x16, b'u')),
        Key::V => Some((0x2F, b'v')),
        Key::W => Some((0x11, b'w')),
        Key::X => Some((0x2D, b'x')),
        Key::Y => Some((0x15, b'y')),
        Key::Z => Some((0x2C, b'z')),
        Key::Key0 => Some((0x0B, b'0')),
        Key::Key1 => Some((0x02, b'1')),
        Key::Key2 => Some((0x03, b'2')),
        Key::Key3 => Some((0x04, b'3')),
        Key::Key4 => Some((0x05, b'4')),
        Key::Key5 => Some((0x06, b'5')),
        Key::Key6 => Some((0x07, b'6')),
        Key::Key7 => Some((0x08, b'7')),
        Key::Key8 => Some((0x09, b'8')),
        Key::Key9 => Some((0x0A, b'9')),
        Key::Minus => Some((0x0C, b'-')),
        Key::Equal => Some((0x0D, b'=')),
        Key::LeftBracket => Some((0x1A, b'[')),
        Key::RightBracket => Some((0x1B, b']')),
        Key::Semicolon => Some((0x27, b';')),
        Key::Apostrophe => Some((0x28, b'\'')),
        Key::Backslash => Some((0x2B, b'\\')),
        Key::Comma => Some((0x33, b',')),
        Key::Period => Some((0x34, b'.')),
        Key::Slash => Some((0x35, b'/')),
        Key::Enter => Some((0x1C, 0x0D)),
        Key::Escape => Some((0x01, 0x1B)),
        Key::Backspace => Some((0x0E, 0x08)),
        Key::Tab => Some((0x0F, 0x09)),
        Key::Space => Some((0x39, b' ')),
        // Стрелки и навигация (ASCII = 0, только scancode)
        Key::Up => Some((0x48, 0)),
        Key::Down => Some((0x50, 0)),
        Key::Left => Some((0x4B, 0)),
        Key::Right => Some((0x4D, 0)),
        Key::Home => Some((0x47, 0)),
        Key::End => Some((0x4F, 0)),
        Key::PageUp => Some((0x49, 0)),
        Key::PageDown => Some((0x51, 0)),
        Key::Insert => Some((0x52, 0)),
        Key::Delete => Some((0x53, 0)),
        // F1-F10
        Key::F1 => Some((0x3B, 0)),
        Key::F2 => Some((0x3C, 0)),
        Key::F3 => Some((0x3D, 0)),
        Key::F4 => Some((0x3E, 0)),
        Key::F5 => Some((0x3F, 0)),
        Key::F6 => Some((0x40, 0)),
        Key::F7 => Some((0x41, 0)),
        Key::F8 => Some((0x42, 0)),
        Key::F9 => Some((0x43, 0)),
        Key::F10 => Some((0x44, 0)),
        // Модификаторы
        Key::LeftShift | Key::RightShift => Some((0x2A, 0)),
        Key::LeftCtrl | Key::RightCtrl => Some((0x1D, 0)),
        Key::LeftAlt | Key::RightAlt => Some((0x38, 0)),
        _ => None,
    }
}
