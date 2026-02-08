// Ver: 4

use crate::DosMachine;

#[derive(Debug, Clone, Copy)]
pub struct ModRm {
    pub mod_field: u8,
    pub reg_field: u8,
    pub rm_field: u8,
}

impl ModRm {
    #[inline]
    pub fn from_byte(byte: u8) -> Self {
        Self {
            mod_field: (byte >> 6) & 0x3,
            reg_field: (byte >> 3) & 0x7,
            rm_field: byte & 0x7,
        }
    }

    #[inline]
    pub fn is_register_mode(&self) -> bool {
        self.mod_field == 0b11
    }

    pub fn resolve_address(
        &self,
        machine: &mut DosMachine,
        addr32_mode: bool,
        bytes: &mut Vec<u8>,
    ) -> Option<u32> {
        if self.is_register_mode() {
            return None;
        }

        let segment = machine.override_segment.unwrap_or(machine.registers.ds());

        if addr32_mode {
            let base = match self.rm_field {
                0 => machine.registers.eax(),
                1 => machine.registers.ecx(),
                2 => machine.registers.edx(),
                3 => machine.registers.ebx(),
                4 => 0,
                5 => {
                    if self.mod_field == 0 {
                        // [disp32]
                        let disp = machine.read_u32(machine.registers.cs(), machine.registers.ip());
                        machine.registers.step(Some(4));
                        return Some(((segment as u32) << 4).wrapping_add(disp) & 0xFFFFF);
                    } else {
                        machine.registers.esp()
                    }
                }
                6 => machine.registers.esi(),
                7 => machine.registers.edi(),
                _ => unreachable!(),
            };

            let disp = match self.mod_field {
                0 => 0,
                1 => {
                    let d = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8
                        as i32;
                    machine.registers.step(None);
                    d
                }
                2 => {
                    let d = machine.read_u32(machine.registers.cs(), machine.registers.ip());
                    machine.registers.step(Some(4));
                    d as i32
                }
                _ => unreachable!(),
            };

            let linear = (base as i32).wrapping_add(disp) as u32;
            Some(((segment as u32) << 4).wrapping_add(linear) & 0xFFFFF)
        } else {
            let (base, index) = match self.rm_field {
                0 => (machine.registers.bx(), machine.registers.si()),
                1 => (machine.registers.bx(), machine.registers.di()),
                2 => (machine.registers.bp(), machine.registers.si()),
                3 => (machine.registers.bp(), machine.registers.di()),
                4 => (0, machine.registers.si()),
                5 => (0, machine.registers.di()),
                6 => {
                    if self.mod_field == 0 {
                        let disp = machine.read_u16(machine.registers.cs(), machine.registers.ip());
                        machine.registers.step(Some(2));
                        bytes.extend_from_slice(&disp.to_le_bytes());

                        let seg = if self.rm_field == 6 && self.mod_field == 0 {
                            machine.registers.ds()
                        } else {
                            segment
                        };
                        return Some(((seg as u32) << 4).wrapping_add(disp as u32) & 0xFFFFF);
                    } else {
                        (machine.registers.bp(), 0)
                    }
                }
                7 => (machine.registers.bx(), 0),
                _ => unreachable!(),
            };

            let disp = match self.mod_field {
                0 => 0,
                1 => {
                    let d = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8
                        as i16;
                    machine.registers.step(None);
                    d as i32
                }
                2 => {
                    let d = machine.read_u16(machine.registers.cs(), machine.registers.ip());
                    machine.registers.step(Some(2));
                    d as i32
                }
                _ => unreachable!(),
            };

            let effective = (base as i32 + index as i32 + disp) as u32;
            let uses_bp_or_sp = self.rm_field == 5
                || self.rm_field == 2
                || self.rm_field == 3
                || (self.rm_field == 6 && self.mod_field != 0);
            let effective_segment = if uses_bp_or_sp {
                machine.registers.ss()
            } else {
                segment
            };

            Some(((effective_segment as u32) << 4).wrapping_add(effective) & 0xFFFFF)
        }
    }
}
