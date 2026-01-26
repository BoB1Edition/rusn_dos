use log::error;

use crate::{machine::DosMachine, modrm::ModRm};

pub fn shift_group_c1(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
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

    let modrm = ModRm::from_byte(modrm_byte);

    let value = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u32(addr)
    };
    let (result, new_flags) =
        perform_shift(modrm.reg_field, value, imm8, machine.registers.flags());
    machine.write_reg32(modrm.rm_field, result);
    machine.registers.set_flags(new_flags);

    machine.log_instruction(csip, &bytes).ok();
}

fn perform_shift(op_field: u8, value: u32, count: u8, flags: u16) -> (u32, u16) {
    let count = count & 0x1F; // x86: count mod 32
    if count == 0 {
        // Никаких изменений, флаги не меняются
        return (value, flags);
    }

    let mut new_flags = flags;
    let af = false;
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
            new_flags = DosMachine::compute_flags_u32(result, cf, of, af);
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
            new_flags = DosMachine::compute_flags_u32(result, cf, of, af);
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
            new_flags = DosMachine::compute_flags_u32(result, new_cf, of, af);
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
            new_flags = DosMachine::compute_flags_u32(result, new_cf, of, af);
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
            new_flags = DosMachine::compute_flags_u32(result, cf, of, af);
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
            new_flags = DosMachine::compute_flags_u32(result, cf, of, af);
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
            new_flags = DosMachine::compute_flags_u32(result, cf, of, af);
            result
        }
        _ => unreachable!(),
    };

    (result, new_flags)
}

pub fn or(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        // OR reg32, reg32 — 0x67 игнорируется
        let src = machine.read_reg32(modrm.reg_field);
        let dst = machine.read_reg32(modrm.rm_field);
        let result = dst | src;
        machine.write_reg32(modrm.rm_field, result);
        let mut flags = machine.registers.flags();
        flags &= !(1 << 0 | 1 << 2 | 1 << 6 | 1 << 7 | 1 << 11);
        if result == 0 {
            flags |= 1 << 6;
        }
        if (result & 0x8000_0000) != 0 {
            flags |= 1 << 7;
        }
        if (result as u8).count_ones() % 2 == 0 {
            flags |= 1 << 2;
        }
        machine.registers.set_flags(flags);
    } else if modrm.mod_field == 0b00 && modrm.rm_field == 0b101 {
        if !machine.has_address_size_prefix {
            log::error!("Invalid memory mode for OR without address-size prefix");
            machine.halted = true;
            return;
        }
        let addr = machine.read_u32(machine.registers.cs(), machine.registers.ip());
        machine.registers.step(Some(4));
        bytes.extend_from_slice(&addr.to_le_bytes());
        let src_val = machine.read_reg32(modrm.reg_field);
        let dst_val = machine.read_phys_u32(addr as u32);
        let result = dst_val | src_val;
        machine.write_phys_u32(addr as u32, result);
        let mut flags = machine.registers.flags();
        flags &= !(1 << 0 | 1 << 2 | 1 << 6 | 1 << 7 | 1 << 11);
        if result == 0 {
            flags |= 1 << 6;
        }
        if (result & 0x8000_0000) != 0 {
            flags |= 1 << 7;
        }
        if (result as u8).count_ones() % 2 == 0 {
            flags |= 1 << 2;
        }
        machine.registers.set_flags(flags);
    } else {
        log::error!("Unsupported memory mode in OR r/m32, r32");
        machine.halted = true;
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub fn add_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        // ADD reg32, reg32
        let src = machine.read_reg32(modrm.reg_field);
        let dst = machine.read_reg32(modrm.rm_field);
        let res = (dst as u64) + (src as u64);
        let result = res as u32;
        let cf = res > 0xFFFFFFFF;
        let af = ((dst & 0x0F) + (src & 0x0F)) > 0x0F;
        let of = (((dst ^ src) & 0x8000_0000) == 0) && ((dst ^ result) & 0x8000_0000) != 0;
        machine.write_reg32(modrm.rm_field, result);
        // Установка флагов...
        let mut flags = machine.registers.flags();
        flags &= !(1 << 0 | 1 << 4 | 1 << 6 | 1 << 7 | 1 << 11);
        if result == 0 {
            flags |= 1 << 6;
        }
        if (result & 0x8000_0000) != 0 {
            flags |= 1 << 7;
        }
        if cf {
            flags |= 1 << 0;
        }
        if of {
            flags |= 1 << 11;
        }
        if af {
            flags |= 1 << 4;
        }
        machine.registers.set_flags(flags);
    } else if modrm.mod_field == 0b00 && modrm.rm_field == 0b110 {
        // [disp16] — разрешено ТОЛЬКО если НЕТ 0x67
        if machine.has_address_size_prefix {
            log::error!("Invalid memory mode for ADD with address-size prefix");
            machine.halted = true;
            return;
        }
        let disp16 = machine.read_u16(machine.registers.cs(), machine.registers.ip());
        machine.registers.step(Some(2));
        bytes.extend_from_slice(&disp16.to_le_bytes());
        let phys_addr = (machine.registers.ds() as u32) * 16 + (disp16 as u32);
        let dst_val = machine.read_phys_u32(phys_addr);
        let src_val = machine.read_reg32(modrm.reg_field);
        let res = (dst_val as u64) + (src_val as u64);
        let result = res as u32;
        let cf = res > 0xFFFFFFFF;
        let af = ((dst_val & 0x0F) + (src_val & 0x0F)) > 0x0F;
        let of =
            (((dst_val ^ src_val) & 0x8000_0000) == 0) && ((dst_val ^ result) & 0x8000_0000) != 0;
        machine.write_phys_u32(phys_addr, result);
        // Установка флагов...
        let mut flags = machine.registers.flags();
        flags &= !(1 << 0 | 1 << 4 | 1 << 6 | 1 << 7 | 1 << 11);
        if result == 0 {
            flags |= 1 << 6;
        }
        if (result & 0x8000_0000) != 0 {
            flags |= 1 << 7;
        }
        if cf {
            flags |= 1 << 0;
        }
        if of {
            flags |= 1 << 11;
        }
        if af {
            flags |= 1 << 4;
        }
        machine.registers.set_flags(flags);
    } else {
        log::error!("Unsupported memory mode in ADD r/m32, r32");
        machine.halted = true;
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub fn sub_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    machine.log_instruction(csip, &bytes).ok();

    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        // SUB reg32, reg32
        let dst_val = machine.read_reg32(modrm.reg_field);
        let src_val = machine.read_reg32(modrm.rm_field);
        let res = (dst_val as i64) - (src_val as i64);
        let result = res as u32;
        let cf = dst_val < src_val;
        let af = (dst_val & 0x0F) < (src_val & 0x0F);
        let of =
            (((dst_val ^ src_val) & 0x8000_0000) != 0) && (((dst_val ^ result) & 0x8000_0000) != 0);
        machine.write_reg32(modrm.reg_field, result);

        let mut flags = machine.registers.flags();
        flags &= !(1 << 0 | 1 << 2 | 1 << 4 | 1 << 6 | 1 << 7 | 1 << 11);
        if result == 0 {
            flags |= 1 << 6;
        }
        if (result & 0x8000_0000) != 0 {
            flags |= 1 << 7;
        }
        if (result as u8).count_ones() % 2 == 0 {
            flags |= 1 << 2;
        }
        if cf {
            flags |= 1 << 0;
        }
        if of {
            flags |= 1 << 11;
        }
        if af {
            flags |= 1 << 4;
        }
        machine.registers.set_flags(flags);
    } else {
        log::error!("Memory operand in SUB r32, r/m32 not supported yet");
        machine.halted = true;
    }
}


pub fn add_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);
    let src_val = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field) // источник: r/m32
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u32(addr)
    };

    let dst_reg = modrm.reg_field; // приёмник: r32
    let dst_val = machine.read_reg32(dst_reg);

    let res = (dst_val as u64) + (src_val as u64);
    let result = res as u32;
    let cf = res > 0xFFFFFFFF;
    let af = ((dst_val & 0x0F) + (src_val & 0x0F)) > 0x0F;
    let of = (((dst_val ^ src_val) & 0x8000_0000) == 0) && ((dst_val ^ result) & 0x8000_0000) != 0;

    machine.write_reg32(dst_reg, result);
    machine.registers.set_flags(DosMachine::compute_flags_u32(result, cf, of, af));

    machine.log_instruction(csip, &bytes).ok();
}