// Ver: 1 File: ./crate/x86/src/modrm.rs
use crate::cpu::X86Cpu;
use bus::Machine;

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
        cpu: &mut X86Cpu,
        machine: &mut dyn Machine,
        addr32_mode: bool,
    ) -> u32 {
        if self.is_register_mode() {
            return 0;
        }

        if addr32_mode {
            self.resolve_32(cpu, machine)
        } else {
            self.resolve_16(cpu, machine)
        }
    }

    fn resolve_16(&self, cpu: &mut X86Cpu, machine: &mut dyn Machine) -> u32 {
        let (base, index) = match self.rm_field {
            0 => (cpu.registers.bx(), cpu.registers.si()),
            1 => (cpu.registers.bx(), cpu.registers.di()),
            2 => (cpu.registers.bp(), cpu.registers.si()),
            3 => (cpu.registers.bp(), cpu.registers.di()),
            4 => (0, cpu.registers.si()),
            5 => (0, cpu.registers.di()),
            6 => {
                if self.mod_field == 0 {
                    let disp = cpu.fetch_u16(machine);
                    let seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
                    return cpu.phys_addr(seg, disp);
                }
                (cpu.registers.bp(), 0)
            }
            7 => (cpu.registers.bx(), 0),
            _ => unreachable!(),
        };

        let disp: i16 = match self.mod_field {
            0 => 0,
            1 => cpu.fetch_u8(machine) as i8 as i16,
            2 => cpu.fetch_u16(machine) as i16,
            _ => unreachable!(),
        };

        let offset = base.wrapping_add(index).wrapping_add(disp as u16);
        let uses_bp =
            self.rm_field == 2 || self.rm_field == 3 || (self.rm_field == 6 && self.mod_field != 0);
        let default_seg = if uses_bp {
            cpu.registers.ss()
        } else {
            cpu.registers.ds()
        };
        let seg = cpu.prefixes.segment_override.unwrap_or(default_seg);

        cpu.phys_addr(seg, offset)
    }

    fn resolve_32(&self, cpu: &mut X86Cpu, machine: &mut dyn Machine) -> u32 {
        let mut base_val = 0u32;
        let mut scaled_index = 0u32;
        let mut ss_default = false;

        if self.rm_field == 4 {
            let sib = cpu.fetch_u8(machine);
            let scale = ((sib >> 6) & 0x3) as u32;
            let index_field = (sib >> 3) & 0x7;
            let base_field = sib & 0x7;
            ss_default = base_field == 4 || base_field == 5;

            if index_field != 4 {
                let idx_reg = match index_field {
                    0 => cpu.registers.eax(),
                    1 => cpu.registers.ecx(),
                    2 => cpu.registers.edx(),
                    3 => cpu.registers.ebx(),
                    5 => cpu.registers.ebp(),
                    6 => cpu.registers.esi(),
                    7 => cpu.registers.edi(),
                    _ => unreachable!(),
                };
                scaled_index = idx_reg.wrapping_shl(scale);
            }

            if base_field == 5 && self.mod_field == 0 {
                let disp = cpu.fetch_u32(machine);
                let seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
                return cpu.phys_addr32(seg, scaled_index.wrapping_add(disp));
            } else {
                base_val = match base_field {
                    0 => cpu.registers.eax(),
                    1 => cpu.registers.ecx(),
                    2 => cpu.registers.edx(),
                    3 => cpu.registers.ebx(),
                    4 => cpu.registers.esp(),
                    5 => cpu.registers.ebp(),
                    6 => cpu.registers.esi(),
                    7 => cpu.registers.edi(),
                    _ => unreachable!(),
                };
            }
        } else {
            if self.rm_field == 5 && self.mod_field == 0 {
                let disp = cpu.fetch_u32(machine);
                let seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
                return cpu.phys_addr32(seg, disp);
            }
            base_val = match self.rm_field {
                0 => cpu.registers.eax(),
                1 => cpu.registers.ecx(),
                2 => cpu.registers.edx(),
                3 => cpu.registers.ebx(),
                4 => cpu.registers.esp(),
                5 => cpu.registers.ebp(),
                6 => cpu.registers.esi(),
                7 => cpu.registers.edi(),
                _ => unreachable!(),
            };
        }

        let disp: i32 = match self.mod_field {
            0 => 0,
            1 => cpu.fetch_u8(machine) as i8 as i32,
            2 => cpu.fetch_u32(machine) as i32,
            _ => unreachable!(),
        };

        let effective = base_val
            .wrapping_add(scaled_index)
            .wrapping_add(disp as u32);

        let default_seg = if ss_default || (self.rm_field == 5 && self.mod_field != 0) {
            cpu.registers.ss()
        } else {
            cpu.registers.ds()
        };
        let seg = cpu.prefixes.segment_override.unwrap_or(default_seg);

        cpu.phys_addr32(seg, effective)
    }

    pub fn resolve_offset(
        &self,
        cpu: &mut X86Cpu,
        machine: &mut dyn Machine,
        addr32_mode: bool,
    ) -> u32 {
        if self.is_register_mode() {
            return 0;
        }

        if addr32_mode {
            self.resolve_32_no_seg(cpu, machine)
        } else {
            self.resolve_16_offset(cpu, machine) as u32
        }
    }

    fn resolve_32_no_seg(&self, cpu: &mut X86Cpu, machine: &mut dyn Machine) -> u32 {
        let mut base_val = 0u32;
        let mut scaled_index = 0u32;

        if self.rm_field == 4 {
            let sib = cpu.fetch_u8(machine);
            let scale = ((sib >> 6) & 0x3) as u32;
            let index_field = (sib >> 3) & 0x7;
            let base_field = sib & 0x7;

            if index_field != 4 {
                let idx_reg = match index_field {
                    0 => cpu.registers.eax(),
                    1 => cpu.registers.ecx(),
                    2 => cpu.registers.edx(),
                    3 => cpu.registers.ebx(),
                    5 => cpu.registers.ebp(),
                    6 => cpu.registers.esi(),
                    7 => cpu.registers.edi(),
                    _ => unreachable!(),
                };
                scaled_index = idx_reg.wrapping_shl(scale);
            }

            if base_field == 5 && self.mod_field == 0 {
                let disp = cpu.fetch_u32(machine);
                return scaled_index.wrapping_add(disp);
            } else {
                base_val = match base_field {
                    0 => cpu.registers.eax(),
                    1 => cpu.registers.ecx(),
                    2 => cpu.registers.edx(),
                    3 => cpu.registers.ebx(),
                    4 => cpu.registers.esp(),
                    5 => cpu.registers.ebp(),
                    6 => cpu.registers.esi(),
                    7 => cpu.registers.edi(),
                    _ => unreachable!(),
                };
            }
        } else {
            if self.rm_field == 5 && self.mod_field == 0 {
                return cpu.fetch_u32(machine);
            }
            base_val = match self.rm_field {
                0 => cpu.registers.eax(),
                1 => cpu.registers.ecx(),
                2 => cpu.registers.edx(),
                3 => cpu.registers.ebx(),
                4 => cpu.registers.esp(),
                5 => cpu.registers.ebp(),
                6 => cpu.registers.esi(),
                7 => cpu.registers.edi(),
                _ => unreachable!(),
            };
        }

        let disp: i32 = match self.mod_field {
            0 => 0,
            1 => cpu.fetch_u8(machine) as i8 as i32,
            2 => cpu.fetch_u32(machine) as i32,
            _ => unreachable!(),
        };

        base_val.wrapping_add(scaled_index).wrapping_add(disp as u32)
    }

    fn resolve_16_offset(&self, cpu: &mut X86Cpu, machine: &mut dyn Machine) -> u16 {
        let (base, index) = match self.rm_field {
            0 => (cpu.registers.bx(), cpu.registers.si()),
            1 => (cpu.registers.bx(), cpu.registers.di()),
            2 => (cpu.registers.bp(), cpu.registers.si()),
            3 => (cpu.registers.bp(), cpu.registers.di()),
            4 => (0, cpu.registers.si()),
            5 => (0, cpu.registers.di()),
            6 => {
                if self.mod_field == 0 {
                    return cpu.fetch_u16(machine);
                }
                (cpu.registers.bp(), 0)
            }
            7 => (cpu.registers.bx(), 0),
            _ => unreachable!(),
        };

        let disp: i16 = match self.mod_field {
            0 => 0,
            1 => cpu.fetch_u8(machine) as i8 as i16,
            2 => cpu.fetch_u16(machine) as i16,
            _ => unreachable!(),
        };

        base.wrapping_add(index).wrapping_add(disp as u16)
    }
}
