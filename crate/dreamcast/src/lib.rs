// Sega Dreamcast stub
// CPU: Hitachi SH-4 (32-bit RISC) @ 200MHz
// GPU: PowerVR2 (CLX2) — 3D-ускоритель
// Звук: Yamaha AICA (ARM7 + DSP) 64 канала
// Память: 16MB RAM, 8MB VRAM, GD-ROM
// ОС: Windows CE (CE-ROM) или KallistiOS

#![allow(dead_code)]

pub struct DreamcastSystem {
    pub halted: bool,
}

impl DreamcastSystem {
    pub fn new() -> Self {
        log::info!("Sega Dreamcast stub created");
        Self { halted: false }
    }

    pub fn step(&mut self) {
        // TODO: реализовать шаг эмуляции Dreamcast
    }

    pub fn load_gd_image(&mut self, _data: &[u8]) {
        log::info!("Dreamcast GD image loading not yet implemented");
    }
}

impl Default for DreamcastSystem {
    fn default() -> Self {
        Self::new()
    }
}
