// Ver: 1 File: ./crate/bus/src/peripherals/keyboard.rs
use std::collections::VecDeque;

use crate::Peripheral;

#[derive(Debug)]
pub struct Keyboard {
    pub buffer: VecDeque<u16>,
    pub shift_flags: u8,
    pub last_scancode: u8,
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

    pub fn push_key(&mut self, scancode: u8, ascii: u8) {
        self.last_scancode = scancode;
        self.data_ready = true;
        let word = ((scancode as u16) << 8) | (ascii as u16);
        self.buffer.push_back(word);
        if self.buffer.len() > 15 {
            self.buffer.pop_front();
        }
    }
    pub fn read_port_60(&mut self) -> u8 {
        self.data_ready = false;
        self.last_scancode
    }
    pub fn read_port_64(&self) -> u8 {
        let mut status = 0x10;
        if self.data_ready {
            status |= 0x01;
        }
        status
    }
    pub fn has_key(&self) -> bool {
        !self.buffer.is_empty()
    }

    pub fn pop_key(&mut self) -> Option<u16> {
        self.buffer.pop_front()
    }

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