// Ver: 1 File: crate/bus/src/peripherals/video.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoMode {
    Text80x25, // Режим 03h
    Mode13h,   // Режим 13h
}

#[derive(Debug)]
pub struct VideoMemory {
    pub mode: VideoMode,
    pub text_buffer: [u16; 80 * 25], 
    pub frame_buffer: [u8; 320 * 200],
    pub dirty: bool,
}

impl VideoMemory {
    pub fn new() -> Self {
        Self {
            mode: VideoMode::Text80x25,
            text_buffer: [0x0720; 80 * 25],
            frame_buffer: [0u8; 320 * 200],
            dirty: false,
        }
    }

    pub fn set_mode(&mut self, mode: VideoMode) {
        self.mode = mode;
        if mode == VideoMode::Text80x25 {
            self.text_buffer = [0x0720; 80 * 25];
        } else {
            self.frame_buffer = [0u8; 320 * 200];
        }
        self.dirty = true;
    }

    /// Чтение из видеопамяти (MMIO)
    pub fn read_mmio(&self, offset: u32) -> u8 {
        if self.mode == VideoMode::Text80x25 {
            let idx = (offset / 2) as usize;
            if idx < self.text_buffer.len() {
                let word = self.text_buffer[idx];
                return if offset % 2 == 0 { (word & 0xFF) as u8 } else { ((word >> 8) & 0xFF) as u8 };
            }
        } else if self.mode == VideoMode::Mode13h {
            let idx = offset as usize;
            if idx < self.frame_buffer.len() {
                return self.frame_buffer[idx];
            }
        }
        0
    }

    /// Запись в видеопамять (MMIO)
    pub fn write_mmio(&mut self, offset: u32, value: u8) {
        if self.mode == VideoMode::Text80x25 {
            let idx = (offset / 2) as usize;
            if idx < self.text_buffer.len() {
                let word = self.text_buffer[idx];
                self.text_buffer[idx] = if offset % 2 == 0 {
                    (word & 0xFF00) | (value as u16)
                } else {
                    (word & 0x00FF) | ((value as u16) << 8)
                };
                self.dirty = true;
            }
        } else if self.mode == VideoMode::Mode13h {
            let idx = offset as usize;
            if idx < self.frame_buffer.len() {
                self.frame_buffer[idx] = value;
                self.dirty = true;
            }
        }
    }
}