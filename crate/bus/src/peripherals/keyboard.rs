// Ver: 2 File: ./libs/dos_core/src/keyboard.rs
use std::collections::VecDeque;

use crate::Peripheral;

/// Эмулирует контроллер клавиатуры IBM PC (8042) и буфер BIOS
#[derive(Debug)]
pub struct Keyboard {
    /// Буфер сканкодов для INT 16h (старший байт = scancode, младший = ASCII)
    pub buffer: VecDeque<u16>,
    /// Флаги состояния (Shift, Ctrl, Alt, CapsLock) для INT 16h / AH=02h
    pub shift_flags: u8,
    /// Последний прочитанный сканкод (для возврата через порт 0x60)
    pub last_scancode: u8,
    /// Флаг: есть ли непрочитанные данные в порту 0x60 (бит 0 порта 0x64)
    pub data_ready: bool,
}

impl Keyboard {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::with_capacity(16),
            shift_flags: 0,
            last_scancode: 0,
            data_ready: false,
        }
    }

    /// Добавляет нажатие клавиши в буфер BIOS и в порт 0x60
    pub fn push_key(&mut self, scancode: u8, ascii: u8) {
        // 1. Сохраняем для чтения через порт 0x60 (для IRQ1 / INT 09h)
        self.last_scancode = scancode;
        self.data_ready = true;

        // 2. Сохраняем в буфер BIOS для INT 16h
        let word = ((scancode as u16) << 8) | (ascii as u16);
        self.buffer.push_back(word);
        
        // Ограничиваем размер буфера (в реальном BIOS он на 15 клавиш)
        if self.buffer.len() > 15 {
            self.buffer.pop_front();
        }
    }

    /// Чтение порта 0x60 (Output Buffer)
    pub fn read_port_60(&mut self) -> u8 {
        self.data_ready = false; // Сбрасываем флаг "данные готовы"
        self.last_scancode
    }

    /// Чтение порта 0x64 (Status Register)
    pub fn read_port_64(&self) -> u8 {
        let mut status = 0x10; // Базовый статус: система OK, таймауты не происходило
        if self.data_ready {
            status |= 0x01; // Бит 0: Output buffer full (данные готовы для чтения из 0x60)
        }
        status
    }

    /// Проверка: есть ли клавиша в буфере (для INT 16h / AH=01h)
    pub fn has_key(&self) -> bool {
        !self.buffer.is_empty()
    }

    /// Извлечение клавиши из буфера (для INT 16h / AH=00h)
    pub fn pop_key(&mut self) -> Option<u16> {
        self.buffer.pop_front()
    }

    /// Пиковая проверка клавиши без извлечения (для INT 16h / AH=01h)
    pub fn peek_key(&self) -> Option<u16> {
        self.buffer.front().copied()
    }
}

impl Peripheral for Keyboard {
    fn port_read(&mut self, port: u16) -> u8 {
        match port {
            0x60 => self.read_port_60(),
            0x64 => self.read_port_64(),
            _ => 0,
        }
    }
}