// Ver: 1 File: crate/bus/src/motherboard.rs
use crate::{Machine, Memory, Peripheral};
use crate::peripherals::{keyboard::Keyboard, video::VideoMemory, pic::Pic};

pub struct Motherboard {
    pub memory: Memory,
    pub keyboard: Keyboard,
    pub video: VideoMemory,
    pub pic: Pic,
    pub halted: bool,
    pub a20_enabled: bool,
}

impl Motherboard {
    pub fn new(memory_size: usize) -> Self {
        Self {
            memory: Memory::new(memory_size),
            keyboard: Keyboard::new(),
            video: VideoMemory::new(),
            pic: Pic::new(),
            halted: false,
            a20_enabled: false,
        }
    }

    #[inline(always)]
    fn apply_a20_mask(&self, addr: u32) -> u32 {
        if !self.a20_enabled {
            addr & 0xFFFFF // Wrap-around на 1MB
        } else {
            addr
        }
    }
}

impl Machine for Motherboard {
    fn read_mem_u8(&self, addr: u32) -> u8 {
        let masked = self.apply_a20_mask(addr);
        
        // MMIO: Видеопамять (0xA0000 - 0xBFFFF)
        if masked >= 0xA0000 && masked < 0xC0000 {
            let offset = masked - 0xA0000;
            return self.video.read_mmio(offset);
        }
        
        self.memory.read_u8(masked)
    }

    fn write_mem_u8(&mut self, addr: u32, val: u8) {
        let masked = self.apply_a20_mask(addr);
        
        // MMIO: Видеопамять (0xA0000 - 0xBFFFF)
        if masked >= 0xA0000 && masked < 0xC0000 {
            let offset = masked - 0xA0000;
            self.video.write_mmio(offset, val);
            return;
        }
        
        self.memory.write_u8(masked, val);
    }

    fn in_port(&mut self, port: u16) -> u8 {
        match port {
            0x60 | 0x64 => self.keyboard.port_read(port),
            0x20 | 0x21 => self.pic.port_read(port),
            _ => {
                log::trace!("IN port {:#04x} not implemented", port);
                0
            }
        }
    }

    fn out_port(&mut self, port: u16, val: u8) {
        match port {
            0x60 | 0x64 => self.keyboard.port_write(port, val),
            0x20 | 0x21 => self.pic.port_write(port, val),
            0x92 => {
                // Fast A20 Gate
                self.a20_enabled = (val & 0x02) != 0;
            }
            _ => {
                log::trace!("OUT port {:#04x} with {:#04x} not implemented", port, val);
            }
        }
    }

    fn trigger_irq(&mut self, irq_line: u8) {
        self.pic.trigger_irq(irq_line);
    }

    fn is_halted(&self) -> bool {
        self.halted
    }

    fn halt(&mut self) {
        self.halted = true;
    }
}