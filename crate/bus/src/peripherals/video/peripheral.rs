use super::VideoMemory;
use crate::peripheral::Peripheral;

impl Peripheral for VideoMemory {
    fn mem_read(&mut self, addr: u32) -> u8 {
        self.read_mmio(addr)
    }

    fn mem_write(&mut self, addr: u32, val: u8) {
        self.write_mmio(addr, val);
    }

    fn port_read(&mut self, port: u16) -> u8 {
        match port {
            0x3D4 => self.crtc_index,
            0x3D5 => self
                .crtc_regs
                .get(self.crtc_index as usize)
                .copied()
                .unwrap_or(0),
            0x3DA => self.input_status_1(),
            0x3C0 => self.ac_index | ((self.attr_flip_flop as u8) << 5),
            0x3C1 => self
                .ac_regs
                .get(self.ac_index as usize)
                .copied()
                .unwrap_or(0),
            0x3C4 => self.seq_index,
            0x3C5 => self
                .seq_regs
                .get(self.seq_index as usize)
                .copied()
                .unwrap_or(0),
            0x3C6 => self.dac_pixel_mask,
            0x3C7 => {
                if self.dac_mode_read {
                    0x03
                } else {
                    0x00
                }
            }
            0x3C8 => self.dac_write_index,
            0x3C9 => {
                let v = self.dac[self.dac_read_index as usize][self.dac_cycle as usize];
                self.dac_cycle = (self.dac_cycle + 1) % 3;
                if self.dac_cycle == 0 {
                    self.dac_read_index = self.dac_read_index.wrapping_add(1);
                }
                v
            }
            0x3CE => self.grc_index,
            0x3CF => self
                .grc_regs
                .get(self.grc_index as usize)
                .copied()
                .unwrap_or(0),

            0x3C2 => 0x00,
            _ => {
                log::trace!("VGA port read {:#04x} not handled", port);
                0
            }
        }
    }

    fn port_write(&mut self, port: u16, val: u8) {
        match port {
            0x3D4 => self.crtc_index = val,
            0x3D5 => {
                let i = self.crtc_index as usize;
                if i < self.crtc_regs.len() {
                    self.crtc_regs[i] = val;
                    if i == 0x0E || i == 0x0F {
                        self.sync_cursor_from_crtc();
                    }
                    self.dirty = true;
                }
            }
            0x3C0 => {
                if !self.attr_flip_flop {
                    self.ac_index = val & 0x1F;
                } else {
                    let i = self.ac_index as usize;
                    if i < self.ac_regs.len() {
                        self.ac_regs[i] = val;
                        self.dirty = true;
                    }
                }
                self.attr_flip_flop = !self.attr_flip_flop;
            }
            0x3C1 => {}
            0x3C6 => self.dac_pixel_mask = val,
            0x3C7 => {
                self.dac_read_index = val;
                self.dac_cycle = 0;
                self.dac_mode_read = true;
            }
            0x3C8 => {
                self.dac_write_index = val;
                self.dac_cycle = 0;
                self.dac_mode_read = false;
            }
            0x3C9 => {
                self.dac[self.dac_write_index as usize][self.dac_cycle as usize] = val & 0x3F;
                self.dac_cycle = (self.dac_cycle + 1) % 3;
                if self.dac_cycle == 0 {
                    self.dac_write_index = self.dac_write_index.wrapping_add(1);
                }
                self.dirty = true;
            }
            0x3C4 => self.seq_index = val,
            0x3C5 => {
                let idx = self.seq_index as usize;
                if idx < self.seq_regs.len() {
                    self.seq_regs[idx] = val;
                }
            }
            0x3CE => self.grc_index = val,
            0x3CF => {
                let idx = self.grc_index as usize;
                if idx < self.grc_regs.len() {
                    self.grc_regs[idx] = val;
                }
            }
            0x3C2 => {}
            0x3DA => {}
            _ => {
                log::trace!("VGA port write {:#04x} = {:#04x} not handled", port, val);
            }
        }
    }
}
