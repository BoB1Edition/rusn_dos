use crate::cpu::X86Cpu;
use crate::modrm::ModRm;
use bus::Machine;

pub fn shld_imm(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let imm = cpu.fetch_u8(machine) & 0x1F;
    if cpu.prefixes.has_operand_size {
        let src = if modrm.is_register_mode() {
            cpu.read_reg32(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u32(addr)
        };
        let result = if imm > 31 { src } else { src.wrapping_shl(imm as u32) | (cpu.read_reg32(modrm.reg_field) >> (32 - imm)) };
        set_flags_shift(cpu, result, src, imm);
        if modrm.is_register_mode() {
            cpu.write_reg32(modrm.rm_field, result);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u32(addr, result);
        }
    } else {
        let src = if modrm.is_register_mode() {
            cpu.read_reg16(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u16(addr)
        };
        let result = if imm > 15 { src } else { src.wrapping_shl(imm as u32) as u16 | (cpu.read_reg16(modrm.reg_field) >> (16 - imm)) };
        set_flags_shift16(cpu, result, src, imm);
        if modrm.is_register_mode() {
            cpu.write_reg16(modrm.rm_field, result);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u16(addr, result);
        }
    }
}

pub fn shld_cl(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let shift = cpu.registers.cl() & 0x1F;
    if cpu.prefixes.has_operand_size {
        let src = if modrm.is_register_mode() {
            cpu.read_reg32(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u32(addr)
        };
        let result = if shift > 31 { src } else { src.wrapping_shl(shift as u32) | (cpu.read_reg32(modrm.reg_field) >> (32 - shift)) };
        set_flags_shift(cpu, result, src, shift);
        if modrm.is_register_mode() {
            cpu.write_reg32(modrm.rm_field, result);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u32(addr, result);
        }
    } else {
        let src = if modrm.is_register_mode() {
            cpu.read_reg16(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u16(addr)
        };
        let result = if shift > 15 { src } else { src.wrapping_shl(shift as u32) as u16 | (cpu.read_reg16(modrm.reg_field) >> (16 - shift)) };
        set_flags_shift16(cpu, result, src, shift);
        if modrm.is_register_mode() {
            cpu.write_reg16(modrm.rm_field, result);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u16(addr, result);
        }
    }
}

pub fn shrd_imm(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let imm = cpu.fetch_u8(machine) & 0x1F;
    if cpu.prefixes.has_operand_size {
        let src = if modrm.is_register_mode() {
            cpu.read_reg32(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u32(addr)
        };
        let result = if imm > 31 { src } else { src.wrapping_shr(imm as u32) | (cpu.read_reg32(modrm.reg_field) << (32 - imm)) };
        set_flags_shift(cpu, result, src, imm);
        if modrm.is_register_mode() {
            cpu.write_reg32(modrm.rm_field, result);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u32(addr, result);
        }
    } else {
        let src = if modrm.is_register_mode() {
            cpu.read_reg16(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u16(addr)
        };
        let result = if imm > 15 { src } else { src.wrapping_shr(imm as u32) as u16 | (cpu.read_reg16(modrm.reg_field) << (16 - imm)) };
        set_flags_shift16(cpu, result, src, imm);
        if modrm.is_register_mode() {
            cpu.write_reg16(modrm.rm_field, result);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u16(addr, result);
        }
    }
}

pub fn shrd_cl(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let shift = cpu.registers.cl() & 0x1F;
    if cpu.prefixes.has_operand_size {
        let src = if modrm.is_register_mode() {
            cpu.read_reg32(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u32(addr)
        };
        let result = if shift > 31 { src } else { src.wrapping_shr(shift as u32) | (cpu.read_reg32(modrm.reg_field) << (32 - shift)) };
        set_flags_shift(cpu, result, src, shift);
        if modrm.is_register_mode() {
            cpu.write_reg32(modrm.rm_field, result);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u32(addr, result);
        }
    } else {
        let src = if modrm.is_register_mode() {
            cpu.read_reg16(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u16(addr)
        };
        let result = if shift > 15 { src } else { src.wrapping_shr(shift as u32) as u16 | (cpu.read_reg16(modrm.reg_field) << (16 - shift)) };
        set_flags_shift16(cpu, result, src, shift);
        if modrm.is_register_mode() {
            cpu.write_reg16(modrm.rm_field, result);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u16(addr, result);
        }
    }
}

fn set_flags_shift(cpu: &mut X86Cpu, result: u32, src: u32, shift: u8) {
    use crate::flags;
    let mut f = cpu.registers.flags();
    f &= !(flags::CF | flags::OF | flags::SF | flags::ZF | flags::PF);
    f = flags::compute_flags_u32(f, result, false, false, false);
    if shift > 0 {
        let cf = ((src >> (shift as u32 - 1)) & 1) != 0;
        if cf { f |= flags::CF; }
        if shift == 1 {
            let of = ((result >> 31) & 1) != cf as u32;
            if !of { f &= !flags::OF; } else { f |= flags::OF; }
        }
    }
    cpu.registers.set_flags(f);
}

fn set_flags_shift16(cpu: &mut X86Cpu, result: u16, src: u16, shift: u8) {
    use crate::flags;
    let mut f = cpu.registers.flags();
    f &= !(flags::CF | flags::OF | flags::SF | flags::ZF | flags::PF);
    f = flags::compute_flags_u16(f, result, false, false, false);
    if shift > 0 {
        let cf = ((src >> (shift - 1)) & 1) != 0;
        if cf { f |= flags::CF; }
        if shift == 1 {
            let of = ((result >> 15) & 1) != cf as u16;
            if !of { f &= !flags::OF; } else { f |= flags::OF; }
        }
    }
    cpu.registers.set_flags(f);
}

fn bit_offset(value: u16, index: u16) -> (u16, u16) {
    let bit = (value >> (index & 0xF)) & 1;
    (bit as u16, index & 0xF)
}

fn bit_offset32(value: u32, index: u32) -> (u32, u32) {
    let bit = (value >> (index & 0x1F)) & 1;
    (bit, index & 0x1F)
}

pub fn bt(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    if cpu.prefixes.has_operand_size {
        let src = if modrm.is_register_mode() {
            cpu.read_reg32(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u32(addr)
        };
        let index = cpu.read_reg32(modrm.reg_field);
        let (bit, _) = bit_offset32(src, index);
        set_cf(cpu, bit != 0);
    } else {
        let src = if modrm.is_register_mode() {
            cpu.read_reg16(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u16(addr)
        };
        let index = cpu.read_reg16(modrm.reg_field);
        let (bit, _) = bit_offset(src, index);
        set_cf(cpu, bit != 0);
    }
}

pub fn bts(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    if cpu.prefixes.has_operand_size {
        let src = if modrm.is_register_mode() {
            cpu.read_reg32(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u32(addr)
        };
        let index = cpu.read_reg32(modrm.reg_field);
        let (bit, off) = bit_offset32(src, index);
        set_cf(cpu, bit != 0);
        let result = src | (1u32 << off);
        if modrm.is_register_mode() {
            cpu.write_reg32(modrm.rm_field, result);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u32(addr, result);
        }
    } else {
        let src = if modrm.is_register_mode() {
            cpu.read_reg16(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u16(addr)
        };
        let index = cpu.read_reg16(modrm.reg_field);
        let (bit, off) = bit_offset(src, index);
        set_cf(cpu, bit != 0);
        let result = src | (1u16 << off);
        if modrm.is_register_mode() {
            cpu.write_reg16(modrm.rm_field, result);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u16(addr, result);
        }
    }
}

pub fn btr(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    if cpu.prefixes.has_operand_size {
        let src = if modrm.is_register_mode() {
            cpu.read_reg32(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u32(addr)
        };
        let index = cpu.read_reg32(modrm.reg_field);
        let (bit, off) = bit_offset32(src, index);
        set_cf(cpu, bit != 0);
        let result = src & !(1u32 << off);
        if modrm.is_register_mode() {
            cpu.write_reg32(modrm.rm_field, result);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u32(addr, result);
        }
    } else {
        let src = if modrm.is_register_mode() {
            cpu.read_reg16(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u16(addr)
        };
        let index = cpu.read_reg16(modrm.reg_field);
        let (bit, off) = bit_offset(src, index);
        set_cf(cpu, bit != 0);
        let result = src & !(1u16 << off);
        if modrm.is_register_mode() {
            cpu.write_reg16(modrm.rm_field, result);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u16(addr, result);
        }
    }
}

pub fn btc(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    if cpu.prefixes.has_operand_size {
        let src = if modrm.is_register_mode() {
            cpu.read_reg32(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u32(addr)
        };
        let index = cpu.read_reg32(modrm.reg_field);
        let (bit, off) = bit_offset32(src, index);
        set_cf(cpu, bit != 0);
        let result = src ^ (1u32 << off);
        if modrm.is_register_mode() {
            cpu.write_reg32(modrm.rm_field, result);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u32(addr, result);
        }
    } else {
        let src = if modrm.is_register_mode() {
            cpu.read_reg16(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u16(addr)
        };
        let index = cpu.read_reg16(modrm.reg_field);
        let (bit, off) = bit_offset(src, index);
        set_cf(cpu, bit != 0);
        let result = src ^ (1u16 << off);
        if modrm.is_register_mode() {
            cpu.write_reg16(modrm.rm_field, result);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u16(addr, result);
        }
    }
}

pub fn bt_group(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let imm = cpu.fetch_u8(machine);
    if cpu.prefixes.has_operand_size {
        let src = if modrm.is_register_mode() {
            cpu.read_reg32(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u32(addr)
        };
        let index = imm as u32;
        let (bit, off) = bit_offset32(src, index);
        match modrm.reg_field {
            4 => { set_cf(cpu, bit != 0); }
            5 => {
                set_cf(cpu, bit != 0);
                let result = src | (1u32 << off);
                if modrm.is_register_mode() {
                    cpu.write_reg32(modrm.rm_field, result);
                } else {
                    let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
                    machine.write_mem_u32(addr, result);
                }
            }
            6 => {
                set_cf(cpu, bit != 0);
                let result = src & !(1u32 << off);
                if modrm.is_register_mode() {
                    cpu.write_reg32(modrm.rm_field, result);
                } else {
                    let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
                    machine.write_mem_u32(addr, result);
                }
            }
            7 => {
                set_cf(cpu, bit != 0);
                let result = src ^ (1u32 << off);
                if modrm.is_register_mode() {
                    cpu.write_reg32(modrm.rm_field, result);
                } else {
                    let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
                    machine.write_mem_u32(addr, result);
                }
            }
            _ => {
                log::error!("0F BA /{} undefined", modrm.reg_field);
                cpu.halted = true;
            }
        }
    } else {
        let src = if modrm.is_register_mode() {
            cpu.read_reg16(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u16(addr)
        };
        let (bit, off) = bit_offset(src, imm as u16);
        match modrm.reg_field {
            4 => { set_cf(cpu, bit != 0); }
            5 => {
                set_cf(cpu, bit != 0);
                let result = src | (1u16 << off);
                if modrm.is_register_mode() {
                    cpu.write_reg16(modrm.rm_field, result);
                } else {
                    let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
                    machine.write_mem_u16(addr, result);
                }
            }
            6 => {
                set_cf(cpu, bit != 0);
                let result = src & !(1u16 << off);
                if modrm.is_register_mode() {
                    cpu.write_reg16(modrm.rm_field, result);
                } else {
                    let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
                    machine.write_mem_u16(addr, result);
                }
            }
            7 => {
                set_cf(cpu, bit != 0);
                let result = src ^ (1u16 << off);
                if modrm.is_register_mode() {
                    cpu.write_reg16(modrm.rm_field, result);
                } else {
                    let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
                    machine.write_mem_u16(addr, result);
                }
            }
            _ => {
                log::error!("0F BA /{} undefined", modrm.reg_field);
                cpu.halted = true;
            }
        }
    }
}

fn set_cf(cpu: &mut X86Cpu, cf: bool) {
    let mut f = cpu.registers.flags();
    if cf {
        f |= crate::flags::CF;
    } else {
        f &= !crate::flags::CF;
    }
    cpu.registers.set_flags(f);
}
