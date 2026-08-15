// Ver: 4 File: ./libs/dos_core/src/instructions/alu/shift.rs
use crate::{DosMachine, flags, modrm::ModRm};

fn perform_shift_8(op_field: u8, value: u8, count: u8, flags: u16) -> (u8, u16) {
    let count = count & 0x1F;
    if count == 0 {
        return (value, flags);
    }

    let mut new_flags = flags;
    let af = false;

    let result = match op_field {
        0 => {
            // ROL
            let result = value.rotate_left(count as u32);
            let cf = (value >> (8 - count)) & 1 != 0;
            let of = if count == 1 {
                ((value ^ result) & 0x80) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u8(flags, result, cf, of, af);
            result
        }
        1 => {
            // ROR
            let result = value.rotate_right(count as u32);
            let cf = (value >> (count - 1)) & 1 != 0;
            let of = if count == 1 {
                ((result ^ (result >> 1)) & 0x80) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u8(flags, result, cf, of, af);
            result
        }
        2 => {
            // RCL (9 бит)
            let count = (count % 9) as u32;
            if count == 0 {
                return (value, flags);
            }
            let carry_in = (flags & 1) != 0;
            let extended = ((value as u16) << 1) | (carry_in as u16);
            let rotated = (extended << count) | (extended >> (9 - count));
            let result = ((rotated >> 1) & 0xFF) as u8;
            let cf = (rotated & 1) != 0;
            let of = if count == 1 {
                ((value ^ result) & 0x80) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u8(flags, result, cf, of, af);
            result
        }
        3 => {
            // RCR (9 бит)
            let count = (count % 9) as u32;
            if count == 0 {
                return (value, flags);
            }
            let carry_in = (flags & 1) != 0;
            let extended = ((value as u16) << 1) | (carry_in as u16);
            let rotated = (extended >> count) | (extended << (9 - count));
            let result = ((rotated >> 1) & 0xFF) as u8;
            let cf = if count == 1 {
                (value & 1) != 0
            } else {
                ((value >> (count - 1)) & 1) != 0
            };
            let of = if count == 1 {
                ((value ^ result) & 0x80) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u8(flags, result, cf, of, af);
            result
        }
        4 | 6 => {
            // SHL / SAL
            if count >= 8 {
                let cf = if count == 8 { (value & 1) != 0 } else { false };
                new_flags = flags::compute_flags_u8(flags, 0, cf, false, af);
                0
            } else {
                let extended = value as u16;
                let shifted = extended << count;
                let result = shifted as u8;
                let cf = (shifted >> 8) != 0;
                let of = if count == 1 {
                    ((value ^ result) & 0x80) != 0
                } else {
                    false
                };
                new_flags = flags::compute_flags_u8(flags, result, cf, of, af);
                result
            }
        }
        5 => {
            // SHR
            if count >= 8 {
                let cf = if count == 8 { (value >> 7) != 0 } else { false };
                new_flags = flags::compute_flags_u8(flags, 0, cf, false, af);
                0
            } else {
                let shifted = value >> count;
                let result = shifted;
                let cf = (value >> (count - 1)) & 1 != 0;
                let of = if count == 1 {
                    (value & 0x80) != 0
                } else {
                    false
                };
                new_flags = flags::compute_flags_u8(flags, result, cf, of, af);
                result
            }
        }
        7 => {
            // SAR
            if count >= 8 {
                let result = if (value & 0x80) != 0 { 0xFF } else { 0x00 };
                let cf = (result & 1) != 0;
                new_flags = flags::compute_flags_u8(flags, result, cf, false, af);
                result
            } else {
                let shifted = (value as i8).wrapping_shr(count as u32) as u8;
                let result = shifted;
                let cf = (value >> (count - 1)) & 1 != 0;
                let of = false;
                new_flags = flags::compute_flags_u8(flags, result, cf, of, af);
                result
            }
        }
        _ => unreachable!(),
    };

    (result, new_flags)
}

fn perform_shift_16(op_field: u8, value: u16, count: u8, flags: u16) -> (u16, u16) {
    let count = count & 0x1F;
    if count == 0 {
        return (value, flags);
    }

    let mut new_flags = flags;
    let af = false;

    let result = match op_field {
        0 => {
            // ROL (16 бит)
            let count = (count % 16) as u32;
            if count == 0 {
                return (value, flags);
            }
            let result = value.rotate_left(count);
            let cf = (result & 1) != 0;
            let of = if count == 1 {
                ((value ^ result) & 0x8000) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u16(flags, result, cf, of, af);
            result
        }
        1 => {
            // ROR (16 бит)
            let count = (count % 16) as u32;
            if count == 0 {
                return (value, flags);
            }
            let result = value.rotate_right(count);
            let cf = (result & 0x8000) != 0;
            let of = if count == 1 {
                ((result ^ (result >> 1)) & 0x8000) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u16(flags, result, cf, of, af);
            result
        }
        2 => {
            // RCL (17 бит)
            let count = (count % 17) as u32;
            if count == 0 {
                return (value, flags);
            }
            let carry_in = (flags & 1) != 0;
            let extended = ((value as u32) << 1) | (carry_in as u32);
            let rotated = (extended << count) | (extended >> (17 - count));
            let result = ((rotated >> 1) & 0xFFFF) as u16;
            let cf = (rotated & 1) != 0;
            let of = if count == 1 {
                ((value ^ result) & 0x8000) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u16(flags, result, cf, of, af);
            result
        }
        3 => {
            // RCR (17 бит)
            let count = (count % 17) as u32;
            if count == 0 {
                return (value, flags);
            }
            let carry_in = (flags & 1) != 0;
            let extended = ((value as u32) << 1) | (carry_in as u32);
            let rotated = (extended >> count) | (extended << (17 - count));
            let result = ((rotated >> 1) & 0xFFFF) as u16;
            let cf = if count == 1 {
                (value & 1) != 0
            } else {
                ((value >> (count - 1)) & 1) != 0
            };
            let of = if count == 1 {
                ((value ^ result) & 0x8000) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u16(flags, result, cf, of, af);
            result
        }
        4 | 6 => {
            // SHL / SAL
            if count >= 16 {
                let cf = if count == 16 { (value & 1) != 0 } else { false };
                new_flags = flags::compute_flags_u16(flags, 0, cf, false, af);
                0
            } else {
                let extended = value as u32;
                let shifted = extended << count;
                let result = shifted as u16;
                let cf = (shifted >> 16) != 0;
                let of = if count == 1 {
                    ((value ^ result) & 0x8000) != 0
                } else {
                    false
                };
                new_flags = flags::compute_flags_u16(flags, result, cf, of, af);
                result
            }
        }
        5 => {
            // SHR
            if count >= 16 {
                let cf = if count == 16 {
                    (value >> 15) != 0
                } else {
                    false
                };
                new_flags = flags::compute_flags_u16(flags, 0, cf, false, af);
                0
            } else {
                let shifted = value >> count;
                let result = shifted;
                let cf = (value >> (count - 1)) & 1 != 0;
                let of = if count == 1 {
                    (value & 0x8000) != 0
                } else {
                    false
                };
                new_flags = flags::compute_flags_u16(flags, result, cf, of, af);
                result
            }
        }
        7 => {
            // SAR
            if count >= 16 {
                let result = if (value & 0x8000) != 0 {
                    0xFFFF
                } else {
                    0x0000
                };
                let cf = (result & 1) != 0;
                new_flags = flags::compute_flags_u16(flags, result, cf, false, af);
                result
            } else {
                let shifted = (value as i16).wrapping_shr(count as u32) as u16;
                let result = shifted;
                let cf = (value >> (count - 1)) & 1 != 0;
                let of = false;
                new_flags = flags::compute_flags_u16(flags, result, cf, of, af);
                result
            }
        }
        _ => unreachable!(),
    };

    (result, new_flags)
}

pub fn shift_group_c0_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    let imm8 = machine.read_instr_u8(machine.registers.ip().wrapping_add(1));
    machine.registers.step(Some(2));
    bytes.push(modrm_byte);
    bytes.push(imm8);
    let modrm = ModRm::from_byte(modrm_byte);
    let (value, addr_opt) = if modrm.is_register_mode() {
        (machine.read_reg8(modrm.rm_field), None)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u8(addr), Some(addr))
    };
    let (result, new_flags) =
        perform_shift_8(modrm.reg_field, value, imm8, machine.registers.flags());
    if let Some(addr) = addr_opt {
        machine.write_phys_u8(addr, result);
    } else {
        machine.write_reg8(modrm.rm_field, result);
    }
    machine.registers.set_flags(new_flags);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn shift_group_c1_16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    let imm8 = machine.read_instr_u8(machine.registers.ip().wrapping_add(1));
    machine.registers.step(Some(2));
    bytes.push(modrm_byte);
    bytes.push(imm8);
    let modrm = ModRm::from_byte(modrm_byte);
    let (value, addr_opt) = if modrm.is_register_mode() {
        (machine.read_reg16(modrm.rm_field), None)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u16(addr), Some(addr))
    };
    let (result, new_flags) =
        perform_shift_16(modrm.reg_field, value, imm8, machine.registers.flags());
    if let Some(addr) = addr_opt {
        machine.write_phys_u16(addr, result);
    } else {
        machine.write_reg16(modrm.rm_field, result);
    }
    machine.registers.set_flags(new_flags);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn shift_group_d0_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let (value, addr_opt) = if modrm.is_register_mode() {
        (machine.read_reg8(modrm.rm_field), None)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u8(addr), Some(addr))
    };
    let (result, new_flags) = perform_shift_8(modrm.reg_field, value, 1, machine.registers.flags());
    if let Some(addr) = addr_opt {
        machine.write_phys_u8(addr, result);
    } else {
        machine.write_reg8(modrm.rm_field, result);
    }
    machine.registers.set_flags(new_flags);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn shift_group_d1(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let (value, addr_opt) = if modrm.is_register_mode() {
        (machine.read_reg16(modrm.rm_field), None)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u16(addr), Some(addr))
    };
    let (result, new_flags) =
        perform_shift_16(modrm.reg_field, value, 1, machine.registers.flags());
    if let Some(addr) = addr_opt {
        machine.write_phys_u16(addr, result);
    } else {
        machine.write_reg16(modrm.rm_field, result);
    }
    machine.registers.set_flags(new_flags);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn shift_rm8_cl(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let count = machine.registers.cl() & 0x1F;
    let (value, addr_opt) = if modrm.is_register_mode() {
        (machine.read_reg8(modrm.rm_field), None)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u8(addr), Some(addr))
    };
    let (result, new_flags) =
        perform_shift_8(modrm.reg_field, value, count, machine.registers.flags());
    if let Some(addr) = addr_opt {
        machine.write_phys_u8(addr, result);
    } else {
        machine.write_reg8(modrm.rm_field, result);
    }
    machine.registers.set_flags(new_flags);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn shift_rm16_cl(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let count = machine.registers.cl() & 0x1F;
    let (value, addr_opt) = if modrm.is_register_mode() {
        (machine.read_reg16(modrm.rm_field), None)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u16(addr), Some(addr))
    };
    let (result, new_flags) =
        perform_shift_16(modrm.reg_field, value, count, machine.registers.flags());
    if let Some(addr) = addr_opt {
        machine.write_phys_u16(addr, result);
    } else {
        machine.write_reg16(modrm.rm_field, result);
    }
    machine.registers.set_flags(new_flags);
    machine.log_instruction(csip, &bytes).ok();
}
