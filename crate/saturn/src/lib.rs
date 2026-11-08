// Sega Saturn stub
// CPU: 2× Hitachi SH-2 (32-bit RISC) @ 28.6MHz
// GPU: 2× VDP (VDP1/VDP2)
// SCU: системный контроллер + DSP
// Звук: SCSP (Sega Custom Sound Processor) 32 канала
// Память: 2MB RAM, 1.5MB VRAM, CD-ROM

#![allow(dead_code)]

pub struct SaturnSystem {
    pub halted: bool,
}

impl SaturnSystem {
    pub fn new() -> Self {
        log::info!("Sega Saturn stub created");
        Self { halted: false }
    }

    pub fn step(&mut self) {
        // TODO: реализовать шаг эмуляции Saturn
    }

    pub fn load_cd_image(&mut self, _data: &[u8]) {
        log::info!("Saturn CD image loading not yet implemented");
    }
}

impl Default for SaturnSystem {
    fn default() -> Self {
        Self::new()
    }
}
