use log::error;

use crate::{machine::DosMachine, modrm::ModRm};

pub fn shift_group_c1(machine: &mut DosMachine, prev: &[u8]) {
    let mut bytes = prev.to_vec();
    //if !machine.has_address_size_prefix {
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    let imm8 = machine.read_u8(
        machine.registers.cs(),
        machine.registers.ip().wrapping_add(1),
    );
    machine.registers.step(Some(2));
    bytes.push(modrm_byte);
    bytes.push(imm8);
    machine.log_instruction(&bytes).ok();

    let modrm = ModRm::from_byte(modrm_byte);
    if !modrm.is_register_mode() {
        log::error!("Memory operand in 0xC1 not supported");
        machine.halted = true;
        return;
    }

    let value = machine.read_reg32(modrm.rm_field);
    let (result, new_flags) =
        perform_shift(modrm.reg_field, value, imm8, machine.registers.flags());
    machine.write_reg32(modrm.rm_field, result);
    machine.registers.set_flags(new_flags);
}

fn perform_shift(op_field: u8, value: u32, count: u8, flags: u16) -> (u32, u16) {
    let count = count & 0x1F; // x86: count mod 32
    if count == 0 {
        // Никаких изменений, флаги не меняются
        return (value, flags);
    }

    let mut new_flags = flags;

    let result = match op_field {
        0 => {
            // ROL
            let result = value.rotate_left(count as u32);
            let cf = (result >> 31) != 0;
            let msb_before = (value >> 31) != 0;
            let msb_after = (result >> 31) != 0;
            let of = if count == 1 {
                msb_before != msb_after
            } else {
                false
            };
            update_flags(&mut new_flags, result, cf, of);
            result
        }
        1 => {
            // ROR
            let result = value.rotate_right(count as u32);
            let cf = (result & 1) != 0;
            let lsb_before = (value & 1) != 0;
            let msb_after = (result >> 31) != 0;
            let of = if count == 1 {
                lsb_before != msb_after
            } else {
                false
            };
            update_flags(&mut new_flags, result, cf, of);
            result
        }
        2 => {
            // RCL - Rotate Left through Carry
            let carry_in = (flags & 1) != 0;
            let input = (value as u64) | ((carry_in as u64) << 32);
            let rotated = input.rotate_left(count as u32);
            let result = rotated as u32;
            let new_cf = (rotated >> 32) != 0;
            let msb_before = (value >> 31) != 0;
            let msb_after = (result >> 31) != 0;
            let of = if count == 1 {
                msb_before != msb_after
            } else {
                false
            };
            update_flags(&mut new_flags, result, new_cf, of);
            result
        }
        3 => {
            // RCR - Rotate Right through Carry
            let carry_in = (flags & 1) != 0;
            let input = (value as u64) | ((carry_in as u64) << 32);
            let rotated = input.rotate_right(count as u32);
            let result = rotated as u32;
            let new_cf = (rotated >> 32) != 0;
            let of = if count == 1 {
                let msb = (result >> 31) & 1;
                let second_msb = (result >> 30) & 1;
                msb != second_msb
            } else {
                false
            };
            update_flags(&mut new_flags, result, new_cf, of);
            result
        }
        4 | 6 => {
            // SHL / SAL - Shift Left Arithmetic/Logical (одинаково)
            let extended = value as u64;
            let shifted = extended << count;
            let result = shifted as u32;
            let cf = (shifted >> 32) != 0;
            let of = if count == 1 {
                let msb_result = (result >> 31) & 1;
                let cf_bit = cf as u32;
                (msb_result ^ cf_bit) != 0
            } else {
                false
            };
            update_flags(&mut new_flags, result, cf, of);
            result
        }
        5 => {
            // SHR - Shift Right Logical
            let shifted = value >> count;
            let result = shifted;
            let cf = (value >> (count - 1)) & 1 != 0;
            let of = if count == 1 {
                (((result >> 31) ^ (cf as u32)) & 1) != 0
            } else {
                false
            };
            update_flags(&mut new_flags, result, cf, of);
            result
        }
        7 => {
            // SAR - Shift Right Arithmetic
            let shifted = (value as i32).wrapping_shr(count as u32) as u32;
            let result = shifted;
            let cf = if count == 0 {
                false
            } else {
                (value >> (count - 1)) & 1 != 0
            };
            let of = false; // OF cleared for SAR
            update_flags(&mut new_flags, result, cf, of);
            result
        }
        _ => unreachable!(),
    };

    (result, new_flags)
}

fn update_flags(flags: &mut u16, result: u32, cf: bool, of: bool) {
    *flags &= !(1 << 0 | 1 << 2 | 1 << 6 | 1 << 7 | 1 << 11);
    if cf {
        *flags |= 1 << 0;
    }
    if of {
        *flags |= 1 << 11;
    }
    if result == 0 {
        *flags |= 1 << 6;
    }
    if (result & 0x8000) != 0 {
        *flags |= 1 << 7;
    }
    if (result as u8).count_ones() % 2 == 0 {
        *flags |= 1 << 2;
    }
}

pub fn or(machine: &mut DosMachine, prev: &[u8]) {
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None); // IP += 1 (ModR/M прочитан)

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        // Регистровый режим: OR r32, r32
        let src = machine.read_reg32(modrm.reg_field);
        let dst = machine.read_reg32(modrm.rm_field);
        let result = dst | src;
        machine.write_reg32(modrm.rm_field, result);

        // Установка флагов
        let mut flags = machine.registers.flags();
        flags &= !(1 << 0 | 1 << 2 | 1 << 6 | 1 << 7 | 1 << 11); // CF=0, OF=0
        if result == 0 { flags |= 1 << 6; } // ZF
        if (result & 0x8000_0000) != 0 { flags |= 1 << 7; } // SF
        if (result as u8).count_ones() % 2 == 0 { flags |= 1 << 2; } // PF
        machine.registers.set_flags(flags);

    } else if modrm.mod_field == 0b00 && modrm.rm_field == 0b101 {
        let addr = machine.read_u32(machine.registers.cs(), machine.registers.ip());
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.registers.step(Some(4)); // IP += 4

        let src_val = machine.read_reg32(modrm.reg_field);
        let dst_val = machine.read_phys_u32(addr as u32);
        let result = dst_val | src_val;

        machine.write_phys_u32(addr as u32, result);
        let mut flags = machine.registers.flags();
        flags &= !(1 << 0 | 1 << 2 | 1 << 6 | 1 << 7 | 1 << 11);
        if result == 0 { flags |= 1 << 6; }
        if (result & 0x8000_0000) != 0 { flags |= 1 << 7; }
        if (result as u8).count_ones() % 2 == 0 { flags |= 1 << 2; }
        machine.registers.set_flags(flags);
    } else {
        machine.log_instruction(&bytes).ok();
        todo!("поддержать другие режимы памяти");
    }
    machine.log_instruction(&bytes).ok();
}
