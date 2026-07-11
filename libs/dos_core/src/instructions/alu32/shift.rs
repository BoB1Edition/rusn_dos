// Ver: 3 File: ./libs/dos_core/src/instructions/alu32/shift.rs
use crate::{DosMachine, flags, modrm::ModRm};

/// Унифицированная функция для всех сдвигов/вращений 32-бит
fn perform_shift_32(op_field: u8, value: u32, count: u8, flags: u16) -> (u32, u16) {
    let count = count & 0x1F; // x86: count mod 32 для всех размеров
    if count == 0 {
        return (value, flags);
    }

    let mut new_flags = flags;
    let af = false;

    let result = match op_field {
        0 => {
            // ROL
            let result = value.rotate_left(count as u32);
            let cf = (value >> (32 - count)) & 1 != 0;
            let of = if count == 1 {
                ((value ^ result) & 0x80000000) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u32(flags, result, cf, of, af);
            result
        }
        1 => {
            // ROR
            let result = value.rotate_right(count as u32);
            let cf = (value >> (count - 1)) & 1 != 0;
            let of = if count == 1 {
                ((result ^ (result >> 1)) & 0x80000000) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u32(flags, result, cf, of, af);
            result
        }
        2 => {
            // RCL (33 бита: 32 бита + CF)
            // ИСПРАВЛЕНИЕ: нормализуем count по модулю 33
            let count = (count % 33) as u32;
            if count == 0 {
                return (value, flags);
            }
            let carry_in = (flags & 1) != 0;
            let extended = ((value as u64) << 1) | (carry_in as u64);
            let rotated = (extended << count) | (extended >> (33 - count));
            let result = ((rotated >> 1) & 0xFFFFFFFF) as u32;
            let cf = (rotated & 1) != 0;
            let of = if count == 1 {
                ((value ^ result) & 0x80000000) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u32(flags, result, cf, of, af);
            result
        }
        3 => {
            // RCR (33 бита: 32 бита + CF)
            // ИСПРАВЛЕНИЕ: нормализуем count по модулю 33
            let count = (count % 33) as u32;
            if count == 0 {
                return (value, flags);
            }
            let carry_in = (flags & 1) != 0;
            let extended = ((value as u64) << 1) | (carry_in as u64);
            let rotated = (extended >> count) | (extended << (33 - count));
            let result = ((rotated >> 1) & 0xFFFFFFFF) as u32;
            let cf = if count == 1 {
                (value & 1) != 0
            } else {
                ((value >> (count - 1)) & 1) != 0
            };
            let of = if count == 1 {
                ((value ^ result) & 0x80000000) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u32(flags, result, cf, of, af);
            result
        }
        4 | 6 => {
            // SHL / SAL
            if count >= 32 {
                let cf = if count == 32 { (value & 1) != 0 } else { false };
                new_flags = flags::compute_flags_u32(flags, 0, cf, false, af);
                0
            } else {
                let extended = value as u64;
                let shifted = extended << count;
                let result = shifted as u32;
                let cf = (shifted >> 32) != 0;
                let of = if count == 1 {
                    ((value ^ result) & 0x80000000) != 0
                } else {
                    false
                };
                new_flags = flags::compute_flags_u32(flags, result, cf, of, af);
                result
            }
        }
        5 => {
            // SHR
            if count >= 32 {
                let cf = if count == 32 {
                    (value >> 31) != 0
                } else {
                    false
                };
                new_flags = flags::compute_flags_u32(flags, 0, cf, false, af);
                0
            } else {
                let shifted = value >> count;
                let result = shifted;
                let cf = (value >> (count - 1)) & 1 != 0;
                let of = if count == 1 {
                    (value & 0x80000000) != 0
                } else {
                    false
                };
                new_flags = flags::compute_flags_u32(flags, result, cf, of, af);
                result
            }
        }
        7 => {
            // SAR
            if count >= 32 {
                let result = if (value & 0x80000000) != 0 {
                    0xFFFFFFFF
                } else {
                    0x00000000
                };
                let cf = (result & 1) != 0;
                new_flags = flags::compute_flags_u32(flags, result, cf, false, af);
                result
            } else {
                let shifted = (value as i32).wrapping_shr(count as u32) as u32;
                let result = shifted;
                let cf = (value >> (count - 1)) & 1 != 0;
                let of = false;
                new_flags = flags::compute_flags_u32(flags, result, cf, of, af);
                result
            }
        }
        _ => unreachable!(),
    };

    (result, new_flags)
}

pub fn shift_group_d1_32(machine: &mut DosMachine, prev: &[u8]) {
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
        (machine.read_reg32(modrm.rm_field), None)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u32(addr), Some(addr))
    };
    let (result, new_flags) =
        perform_shift_32(modrm.reg_field, value, 1, machine.registers.flags());
    if let Some(addr) = addr_opt {
        machine.write_phys_u32(addr, result);
    } else {
        machine.write_reg32(modrm.rm_field, result);
    }
    machine.registers.set_flags(new_flags);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn shift_group_c1_32(machine: &mut DosMachine, prev: &[u8]) {
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
        (machine.read_reg32(modrm.rm_field), None)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u32(addr), Some(addr))
    };
    let (result, new_flags) =
        perform_shift_32(modrm.reg_field, value, imm8, machine.registers.flags());
    if let Some(addr) = addr_opt {
        machine.write_phys_u32(addr, result);
    } else {
        machine.write_reg32(modrm.rm_field, result);
    }
    machine.registers.set_flags(new_flags);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn shift_rm32_cl(machine: &mut DosMachine, prev: &[u8]) {
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
        (machine.read_reg32(modrm.rm_field), None)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u32(addr), Some(addr))
    };
    let (result, new_flags) =
        perform_shift_32(modrm.reg_field, value, count, machine.registers.flags());
    if let Some(addr) = addr_opt {
        machine.write_phys_u32(addr, result);
    } else {
        machine.write_reg32(modrm.rm_field, result);
    }
    machine.registers.set_flags(new_flags);
    machine.log_instruction(csip, &bytes).ok();
}
