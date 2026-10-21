// Ver: 3 File: crate/bus/src/motherboard.rs

use crate::peripherals::{
    keyboard::Keyboard, pic::Pic, pit::Pit, video::VideoMemory,
};
use crate::{Machine, Memory, Peripheral};

pub struct Motherboard {
    pub memory: Memory,
    pub keyboard: Keyboard,
    pub video: VideoMemory,
    pub pic: Pic,
    pub pit: Pit,
    pub halted: bool,
    pub a20_enabled: bool,
    pub a20_command_pending: bool,
}

impl Motherboard {
    pub fn new(memory_size: usize) -> Self {
        Self {
            memory: Memory::new(memory_size),
            keyboard: Keyboard::new(),
            video: VideoMemory::new(),
            pic: Pic::new(),
            pit: Pit::new(),
            halted: false,
            a20_enabled: false,
            a20_command_pending: false,
        }
    }

    #[inline(always)]
    fn apply_a20_mask(&self, addr: u32) -> u32 {
        if !self.a20_enabled {
            addr & 0xFFFFF
        } else {
            addr
        }
    }
}

impl Machine for Motherboard {
    fn read_mem_u8(&self, addr: u32) -> u8 {
        let masked = self.apply_a20_mask(addr);
        if masked >= 0xA0000 && masked < 0xC0000 {
            let offset = masked - 0xA0000;
            return self.video.read_mmio(offset);
        }
        self.memory.read_u8(masked)
    }

    fn write_mem_u8(&mut self, addr: u32, val: u8) {
        let masked = self.apply_a20_mask(addr);
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
            0x20 | 0x21 | 0xA0 | 0xA1 => self.pic.port_read(port),
            0x40..=0x43 => self.pit.port_read(port),
            0x3C2 | 0x3C4 | 0x3C5 | 0x3CE | 0x3CF | 0x3DA => self.video.port_read(port),
            0x92 => {
                if self.a20_enabled {
                    0x02
                } else {
                    0x00
                }
            }
            _ => {
                log::trace!("IN port {:#04x} not implemented", port);
                0
            }
        }
    }

    fn out_port(&mut self, port: u16, val: u8) {
        match port {
            0x60 => {
                if self.a20_command_pending {
                    self.a20_enabled = (val & 0x02) != 0;
                    self.a20_command_pending = false;
                } else {
                    self.keyboard.port_write(port, val);
                }
            }
            0x64 => {
                if val == 0xD1 {
                    self.a20_command_pending = true;
                }
                self.keyboard.port_write(port, val);
            }
            0x20 | 0x21 | 0xA0 | 0xA1 => self.pic.port_write(port, val),
            0x40..=0x43 => self.pit.port_write(port, val),
            0x3C2 | 0x3C4 | 0x3C5 | 0x3CE | 0x3CF => self.video.port_write(port, val),
            0x92 => {
                self.a20_enabled = (val & 0x02) != 0;
            }
            _ => {
                log::trace!("OUT port {:#04x} = {:#04x} not impl", port, val);
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
