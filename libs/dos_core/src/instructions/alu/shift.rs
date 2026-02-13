use crate::{DosMachine, flags, modrm::ModRm};

pub fn shift_group_c1_16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    let imm8 = machine.read_u8(
        machine.registers.cs(),
        machine.registers.ip().wrapping_add(1),
    );
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
    
    let (result, new_flags) = perform_shift_16(modrm.reg_field, value, imm8, machine.registers.flags());
    
    if let Some(addr) = addr_opt {
        // Запись обратно в память
        machine.write_phys_u16(addr, result);
    } else {
        // Запись в регистр
        machine.write_reg16(modrm.rm_field, result);
    }
    
    machine.registers.set_flags(new_flags);
    machine.log_instruction(csip, &bytes).ok();
}

fn perform_shift_16(op_field: u8, value: u16, count: u8, flags: u16) -> (u16, u16) {
    let count = count & 0x0F; // x86: count mod 16 для 16-битных операций
    if count == 0 {
        return (value, flags);
    }
    let mut new_flags = flags;
    let af = false; // AF не определён для сдвигов
    let result = match op_field {
        0 => { // ROL
            let result = value.rotate_left(count as u32);
            let cf = (result & 1) != 0;
            let msb_before = (value >> 15) != 0;
            let msb_after = (result >> 15) != 0;
            let of = if count == 1 { msb_before != msb_after } else { false };
            new_flags = flags::compute_flags_u16(result, cf, of, af);
            result
        }
        1 => { // ROR
            let result = value.rotate_right(count as u32);
            let cf = (result >> 15) != 0;
            let lsb_before = (value & 1) != 0;
            let msb_after = (result >> 15) != 0;
            let of = if count == 1 { lsb_before != msb_after } else { false };
            new_flags = flags::compute_flags_u16(result, cf, of, af);
            result
        }
        2 => { // RCL
            let carry_in = (flags & 1) != 0;
            let input = (value as u32) | ((carry_in as u32) << 16);
            let rotated = input.rotate_left(count as u32);
            let result = rotated as u16;
            let new_cf = (rotated >> 16) != 0;
            let msb_before = (value >> 15) != 0;
            let msb_after = (result >> 15) != 0;
            let of = if count == 1 { msb_before != msb_after } else { false };
            new_flags = flags::compute_flags_u16(result, new_cf, of, af);
            result
        }
        3 => { // RCR
            let carry_in = (flags & 1) != 0;
            let input = (value as u32) | ((carry_in as u32) << 16);
            let rotated = input.rotate_right(count as u32);
            let result = rotated as u16;
            let new_cf = (rotated >> 16) != 0;
            let of = if count == 1 {
                let msb = (result >> 15) & 1;
                let second_msb = (result >> 14) & 1;
                msb != second_msb
            } else {
                false
            };
            new_flags = flags::compute_flags_u16(result, new_cf, of, af);
            result
        }
        4 | 6 => { // SHL / SAL (одинаково)
            let extended = value as u32;
            let shifted = extended << count;
            let result = shifted as u16;
            let cf = (shifted >> 16) != 0;
            let of = if count == 1 {
                let msb_result = (result >> 15) & 1;
                let cf_bit = cf as u16;
                (msb_result ^ cf_bit) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u16(result, cf, of, af);
            result
        }
        5 => { // SHR
            let shifted = value >> count;
            let result = shifted;
            let cf = (value >> (count - 1)) & 1 != 0;
            let of = if count == 1 {
                (((result >> 15) ^ (cf as u16)) & 1) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u16(result, cf, of, af);
            result
        }
        7 => { // SAR
            let shifted = (value as i16).wrapping_shr(count as u32) as u16;
            let result = shifted;
            let cf = if count == 0 {
                false
            } else {
                (value >> (count - 1)) & 1 != 0
            };
            let of = false; // OF cleared for SAR
            new_flags = flags::compute_flags_u16(result, cf, of, af);
            result
        }
        _ => unreachable!(),
    };
    (result, new_flags)
}

pub fn shift_group_d1(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None); // продвигаем только на байт ModR/M
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    if modrm.is_register_mode() {
        // ROL/ROR/RCL/RCR/SHL/SHR/SAR reg16, 1
        let value = machine.read_reg16(modrm.rm_field);
        let (result, new_flags) = perform_shift_16(modrm.reg_field, value, 1, machine.registers.flags());
        machine.write_reg16(modrm.rm_field, result);
        machine.registers.set_flags(new_flags);
    } else {
        // ROL/ROR/RCL/RCR/SHL/SHR/SAR [mem], 1
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let value = machine.read_phys_u16(addr);
        let (result, new_flags) = perform_shift_16(modrm.reg_field, value, 1, machine.registers.flags());
        machine.write_phys_u16(addr, result);
        machine.registers.set_flags(new_flags);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn shift_group_c0_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    let imm8 = machine.read_u8(
        machine.registers.cs(),
        machine.registers.ip().wrapping_add(1),
    );
    machine.registers.step(Some(2));
    bytes.push(modrm_byte);
    bytes.push(imm8);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Для 8-битных операций: счётчик сдвига по модулю 8
    let count = imm8 & 0x07;
    
    if count == 0 {
        // Сдвиг на 0 бит — ничего не делаем (флаги не изменяются)
        machine.log_instruction(csip, &bytes).ok();
        return;
    }
    
    let (value, addr_opt) = if modrm.is_register_mode() {
        (machine.read_reg8(modrm.rm_field), None)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u8(addr), Some(addr))
    };
    
    let (result, new_flags) = perform_shift_8(modrm.reg_field, value, count, machine.registers.flags());
    
    if let Some(addr) = addr_opt {
        // Запись обратно в память
        machine.write_phys_u8(addr, result);
    } else {
        // Запись в регистр
        machine.write_reg8(modrm.rm_field, result);
    }
    
    machine.registers.set_flags(new_flags);
    machine.log_instruction(csip, &bytes).ok();
}

fn perform_shift_8(op_field: u8, value: u8, count: u8, flags: u16) -> (u8, u16) {
    debug_assert!(count > 0 && count < 8, "count must be 1..7 for 8-bit shifts");
    
    let mut new_flags = flags;
    let af = false; // AF не определён для сдвигов
    
    let result = match op_field {
        0 => { // ROL — Rotate Left
            let result = value.rotate_left(count as u32);
            let cf = (result & 1) != 0;
            let of = if count == 1 {
                let msb_before = (value >> 7) != 0;
                let msb_after = (result >> 7) != 0;
                msb_before != msb_after
            } else {
                false
            };
            new_flags = flags::compute_flags_u8(result, cf, of, af);
            result
        }
        1 => { // ROR — Rotate Right
            let result = value.rotate_right(count as u32);
            let cf = (result >> 7) != 0;
            let of = if count == 1 {
                let lsb_before = (value & 1) != 0;
                let msb_after = (result >> 7) != 0;
                lsb_before != msb_after
            } else {
                false
            };
            new_flags = flags::compute_flags_u8(result, cf, of, af);
            result
        }
        2 => { // RCL — Rotate Left through Carry
            let carry_in = (flags & 1) != 0;
            let input = (value as u16) | ((carry_in as u16) << 8);
            let rotated = input.rotate_left(count as u32);
            let result = rotated as u8;
            let new_cf = (rotated >> 8) != 0;
            let of = if count == 1 {
                let msb_before = (value >> 7) != 0;
                let msb_after = (result >> 7) != 0;
                msb_before != msb_after
            } else {
                false
            };
            new_flags = flags::compute_flags_u8(result, new_cf, of, af);
            result
        }
        3 => { // RCR — Rotate Right through Carry
            let carry_in = (flags & 1) != 0;
            let input = (value as u16) | ((carry_in as u16) << 8);
            let rotated = input.rotate_right(count as u32);
            let result = rotated as u8;
            let new_cf = (rotated >> 8) != 0;
            let of = if count == 1 {
                let msb = (result >> 7) & 1;
                let second_msb = (result >> 6) & 1;
                msb != second_msb
            } else {
                false
            };
            new_flags = flags::compute_flags_u8(result, new_cf, of, af);
            result
        }
        4 | 6 => { // SHL / SAL — Shift Left (одинаковы)
            let extended = value as u16;
            let shifted = extended << count;
            let result = shifted as u8;
            let cf = (shifted >> 8) != 0;
            let of = if count == 1 {
                let msb_result = (result >> 7) & 1;
                let cf_bit = cf as u8;
                (msb_result ^ cf_bit) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u8(result, cf, of, af);
            result
        }
        5 => { // SHR — Shift Right Logical
            let shifted = value >> count;
            let result = shifted;
            let cf = (value >> (count - 1)) & 1 != 0;
            let of = if count == 1 {
                let msb_result = (result >> 7) & 1;
                let cf_bit = cf as u8;
                (msb_result ^ cf_bit) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u8(result, cf, of, af);
            result
        }
        7 => { // SAR — Shift Right Arithmetic
            let shifted = (value as i8).wrapping_shr(count as u32) as u8;
            let result = shifted;
            let cf = (value >> (count - 1)) & 1 != 0;
            let of = false; // OF cleared for SAR
            new_flags = flags::compute_flags_u8(result, cf, of, af);
            result
        }
        _ => unreachable!(),
    };
    
    (result, new_flags)
}