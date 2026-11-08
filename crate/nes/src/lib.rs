// NES (Nintendo Entertainment System) stub
// CPU: 6502 кастомный (Ricoh 2A03)
// PPU: 2C02 — графический процессор
// APU: встроенный звук 5 каналов
// Память: 2KB RAM, 2KB VRAM, картриджи до 1MB

#![allow(dead_code)]

pub struct NesSystem {
    pub halted: bool,
}

impl NesSystem {
    pub fn new() -> Self {
        log::info!("NES stub created");
        Self { halted: false }
    }

    pub fn step(&mut self) {
        // TODO: реализовать шаг эмуляции NES
    }

    pub fn load_rom(&mut self, _data: &[u8]) {
        log::info!("NES ROM loading not yet implemented");
    }
}

impl Default for NesSystem {
    fn default() -> Self {
        Self::new()
    }
}
