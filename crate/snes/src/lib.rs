// SNES (Super Nintendo Entertainment System) stub
// CPU: 65816 (WDC 65C816) на 3.58MHz
// PPU1/PPU2: 2 графических чипа
// DSP: звуковой процессор (S-DSP + SPC700)
// Память: 128KB RAM, 64KB VRAM, 512KB картридж (макс 8MB+)

#![allow(dead_code)]

pub struct SnesSystem {
    pub halted: bool,
}

impl SnesSystem {
    pub fn new() -> Self {
        log::info!("SNES stub created");
        Self { halted: false }
    }

    pub fn step(&mut self) {
        // TODO: реализовать шаг эмуляции SNES
    }

    pub fn load_rom(&mut self, _data: &[u8]) {
        log::info!("SNES ROM loading not yet implemented");
    }
}

impl Default for SnesSystem {
    fn default() -> Self {
        Self::new()
    }
}
