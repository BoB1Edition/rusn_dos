// Sega Mega Drive (Genesis) stub
// CPU: Motorola 68000 + Z80 (звуковой сопроцессор)
// VDP: видеопроцессор с 64/512 цветами
// PSG: 4 канала, FM: 6 каналов (YM2612)
// Память: 64KB RAM, 64KB VRAM, картриджи до 4MB

#![allow(dead_code)]

pub struct MegaDriveSystem {
    pub halted: bool,
}

impl MegaDriveSystem {
    pub fn new() -> Self {
        log::info!("Sega Mega Drive stub created");
        Self { halted: false }
    }

    pub fn step(&mut self) {
        // TODO: реализовать шаг эмуляции Mega Drive
    }

    pub fn load_rom(&mut self, _data: &[u8]) {
        log::info!("Mega Drive ROM loading not yet implemented");
    }
}

impl Default for MegaDriveSystem {
    fn default() -> Self {
        Self::new()
    }
}
