// Ver: 5
use crate::{flags, machine::DosMachine, modrm::ModRm};

fn perform_shift(op_field: u8, value: u32, count: u8, flags: u16) -> (u32, u16) {
    let count = count & 0x1F; // x86: count mod 32
    if count == 0 {
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
            new_flags = flags::compute_flags_u32(result, cf, of, af);
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
            new_flags = flags::compute_flags_u32(result, cf, of, af);
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
            new_flags = flags::compute_flags_u32(result, new_cf, of, af);
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
            new_flags = flags::compute_flags_u32(result, new_cf, of, af);
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
            new_flags = flags::compute_flags_u32(result, cf, of, af);
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
            new_flags = flags::compute_flags_u32(result, cf, of, af);
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
            new_flags = flags::compute_flags_u32(result, cf, of, af);
            result
        }
        _ => unreachable!(),
    };

    (result, new_flags)
}

pub fn or_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
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
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
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
    machine.registers.set_flags(flags::compute_flags_u32(result, cf, of, af));

    machine.log_instruction(csip, &bytes).ok();
}

pub fn cmp_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u32(addr)
    };

    let dst_val = machine.read_reg32(modrm.reg_field);
    let res = (dst_val as i64) - (src_val as i64);
    let result = res as u32;
    let cf = dst_val < src_val;
    let af = (dst_val & 0x0F) < (src_val & 0x0F);
    let of = (((dst_val ^ src_val) & 0x8000_0000) != 0) && (((dst_val ^ result) & 0x8000_0000) != 0);

    machine.registers.set_flags(flags::compute_flags_u32(result, cf, of, af));
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/alu32.rs
pub fn add_eax_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm32 = machine.read_u32(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(4));
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm32.to_le_bytes());
    
    let eax = machine.registers.eax();
    let res = (eax as u64) + (imm32 as u64);
    let result = res as u32;
    
    // Установка флагов
    let cf = res > 0xFFFFFFFF;
    let zf = result == 0;
    let sf = (result & 0x8000_0000) != 0;
    let af = ((eax & 0x0F) + (imm32 & 0x0F)) > 0x0F;
    let pf = (result as u8).count_ones() % 2 == 0;
    let of = (((eax ^ imm32) & 0x8000_0000) == 0) && ((eax ^ result) & 0x8000_0000) != 0;
    
    let mut flags = 0u16;
    if cf { flags |= 1 << 0; }
    if pf { flags |= 1 << 2; }
    if af { flags |= 1 << 4; }
    if zf { flags |= 1 << 6; }
    if sf { flags |= 1 << 7; }
    if of { flags |= 1 << 11; }
    machine.registers.set_flags(flags);
    
    machine.registers.set_eax(result);
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/alu32.rs
pub fn shift_group_d1_32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None); // продвигаем только на байт ModR/M
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    if modrm.is_register_mode() {
        // ROL/ROR/RCL/RCR/SHL/SHR/SAR reg32, 1
        let value = machine.read_reg32(modrm.rm_field);
        let (result, new_flags) = perform_shift(modrm.reg_field, value, 1, machine.registers.flags());
        machine.write_reg32(modrm.rm_field, result);
        machine.registers.set_flags(new_flags);
    } else {
        // ROL/ROR/RCL/RCR/SHL/SHR/SAR [mem], 1
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let value = machine.read_phys_u32(addr);
        let (result, new_flags) = perform_shift(modrm.reg_field, value, 1, machine.registers.flags());
        machine.write_phys_u32(addr, result);
        machine.registers.set_flags(new_flags);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/alu32.rs
pub fn group_x83_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    let imm8 = machine.read_u8(
        machine.registers.cs(),
        machine.registers.ip().wrapping_add(1),
    );
    machine.registers.step(Some(2));
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    bytes.push(imm8);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Расширяем imm8 до i8, затем до i32 со знаком (sign-extend)
    let imm32 = (imm8 as i8) as i32 as u32;
    
    if modrm.is_register_mode() {
        // Операция над регистром
        let dst_val = machine.read_reg32(modrm.rm_field);
        let (result, flags) = perform_group_x83_operation_32(modrm.reg_field, dst_val, imm32, machine.registers.flags());
        if modrm.reg_field != 7 { // CMP (reg_field=7) не сохраняет результат
            machine.write_reg32(modrm.rm_field, result);
        }
        machine.registers.set_flags(flags);
    } else {
        // Операция над памятью
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let dst_val = machine.read_phys_u32(addr);
        let (result, flags) = perform_group_x83_operation_32(modrm.reg_field, dst_val, imm32, machine.registers.flags());
        if modrm.reg_field != 7 { // CMP не сохраняет результат
            machine.write_phys_u32(addr, result);
        }
        machine.registers.set_flags(flags);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

fn perform_group_x83_operation_32(op_field: u8, dst_val: u32, imm: u32, flags: u16) -> (u32, u16) {
    match op_field {
        0 => { // ADD r/m32, imm8 (sign-extended)
            let res = dst_val as u64 + imm as u64;
            let result = res as u32;
            let cf = res > 0xFFFFFFFF;
            let af = ((dst_val & 0x0F) + (imm & 0x0F)) > 0x0F;
            let of = (((dst_val ^ imm) & 0x8000_0000) == 0) && ((dst_val ^ result) & 0x8000_0000) != 0;
            (result, flags::compute_flags_u32(result, cf, of, af))
        }
        1 => { // OR r/m32, imm8
            let result = dst_val | imm;
            (result, flags::compute_logical_flags_u32(result))
        }
        2 => { // ADC r/m32, imm8
            let carry_in = (flags & 1) != 0;
            let mut res = dst_val as u64 + imm as u64;
            if carry_in {
                res += 1;
            }
            let result = res as u32;
            let cf = res > 0xFFFFFFFF;
            let af = ((dst_val & 0x0F) + (imm & 0x0F) + if carry_in { 1 } else { 0 }) > 0x0F;
            let of = (((dst_val ^ imm) & 0x8000_0000) == 0) && ((dst_val ^ result) & 0x8000_0000) != 0;
            (result, flags::compute_flags_u32(result, cf, of, af))
        }
        3 => { // SBB r/m32, imm8
            let borrow_in = (flags & 1) != 0;
            let imm_with_borrow = imm as u64 + if borrow_in { 1 } else { 0 };
            let cf = (dst_val as u64) < imm_with_borrow;
            let res = (dst_val as u64).wrapping_sub(imm_with_borrow);
            let result = res as u32;
            let af = (dst_val & 0x0F) < ((imm & 0x0F) + if borrow_in { 1 } else { 0 });
            let of = (((dst_val ^ imm) & 0x8000_0000) != 0) && (((dst_val ^ result) & 0x8000_0000) != 0);
            (result, flags::compute_flags_u32(result, cf, of, af))
        }
        4 => { // AND r/m32, imm8
            let result = dst_val & imm;
            (result, flags::compute_logical_flags_u32(result))
        }
        5 => { // SUB r/m32, imm8
            let res = (dst_val as u64).wrapping_sub(imm as u64);
            let result = res as u32;
            let cf = dst_val < imm;
            let af = (dst_val & 0x0F) < (imm & 0x0F);
            let of = (((dst_val ^ imm) & 0x8000_0000) != 0) && (((dst_val ^ result) & 0x8000_0000) != 0);
            (result, flags::compute_flags_u32(result, cf, of, af))
        }
        6 => { // XOR r/m32, imm8
            let result = dst_val ^ imm;
            (result, flags::compute_logical_flags_u32(result))
        }
        7 => { // CMP r/m32, imm8 — как SUB, но не сохраняем результат
            let res = (dst_val as u64).wrapping_sub(imm as u64);
            let result = res as u32;
            let cf = dst_val < imm;
            let af = (dst_val & 0x0F) < (imm & 0x0F);
            let of = (((dst_val ^ imm) & 0x8000_0000) != 0) && (((dst_val ^ result) & 0x8000_0000) != 0);
            let flags = flags::compute_flags_u32(result, cf, of, af);
            (dst_val, flags) // Возвращаем исходное значение (не сохраняем результат)
        }
        _ => unreachable!(),
    }
}

// libs/dos_core/src/instructions/alu32.rs
pub fn shift_group_c1_32(machine: &mut DosMachine, prev: &[u8]) {
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
        (machine.read_reg32(modrm.rm_field), None)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u32(addr), Some(addr))
    };
    
    let (result, new_flags) = perform_shift(modrm.reg_field, value, imm8, machine.registers.flags());
    
    if let Some(addr) = addr_opt {
        // Запись обратно в память
        machine.write_phys_u32(addr, result);
    } else {
        // Запись в регистр
        machine.write_reg32(modrm.rm_field, result);
    }
    
    machine.registers.set_flags(new_flags);
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/alu32.rs
pub fn or_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Источник: r/m32 (регистр или память)
    let src_val = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u32(addr)
    };
    
    // Приёмник: регистр из reg_field
    let dst_reg = modrm.reg_field;
    let dst_val = machine.read_reg32(dst_reg);
    let result = dst_val | src_val;
    
    // Запись результата в регистр-приёмник
    machine.write_reg32(dst_reg, result);
    
    // Установка флагов (логическая операция: CF=0, OF=0)
    machine.registers.set_flags(flags::compute_logical_flags_u32(result));
    
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/alu32.rs
pub fn and_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Источник: регистр из reg_field
    let src_val = machine.read_reg32(modrm.reg_field);
    
    // Приёмник: r/m32 (регистр или память)
    if modrm.is_register_mode() {
        // AND reg32, reg32
        let dst_val = machine.read_reg32(modrm.rm_field);
        let result = dst_val & src_val;
        machine.write_reg32(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_logical_flags_u32(result));
    } else {
        // AND [mem], reg32
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let dst_val = machine.read_phys_u32(addr);
        let result = dst_val & src_val;
        machine.write_phys_u32(addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u32(result));
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/alu32.rs
pub fn xor_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Источник: регистр из reg_field
    let src_val = machine.read_reg32(modrm.reg_field);
    
    // Приёмник: r/m32 (регистр или память)
    if modrm.is_register_mode() {
        // XOR reg32, reg32
        let dst_val = machine.read_reg32(modrm.rm_field);
        let result = dst_val ^ src_val;
        machine.write_reg32(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_logical_flags_u32(result));
    } else {
        // XOR [mem], reg32
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let dst_val = machine.read_phys_u32(addr);
        let result = dst_val ^ src_val;
        machine.write_phys_u32(addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u32(result));
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/alu32.rs
pub fn xor_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Источник: регистр из reg_field
    let src_val = machine.read_reg32(modrm.reg_field);
    
    // Приёмник: r/m32 (регистр или память)
    if modrm.is_register_mode() {
        // XOR reg32, reg32
        let dst_val = machine.read_reg32(modrm.rm_field);
        let result = dst_val ^ src_val;
        machine.write_reg32(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_logical_flags_u32(result));
    } else {
        // XOR [mem], reg32
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let dst_val = machine.read_phys_u32(addr);
        let result = dst_val ^ src_val;
        machine.write_phys_u32(addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u32(result));
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/alu32.rs
pub fn cmp_eax_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm32 = machine.read_u32(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(4)); // продвигаем на 4 байта (imm32)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm32.to_le_bytes());
    
    let eax = machine.registers.eax();
    
    // Вычисление флагов как при вычитании
    let result = eax.wrapping_sub(imm32);
    
    // Флаг переноса (CF): 1 если беззнаковое переполнение (EAX < imm32)
    let cf = eax < imm32;
    
    // Флаг переполнения (OF): знаковое переполнение при вычитании
    let eax_sign = (eax as i32) < 0;
    let imm_sign = (imm32 as i32) < 0;
    let result_sign = (result as i32) < 0;
    let of = (eax_sign != imm_sign) && (eax_sign != result_sign);
    
    // Флаг вспомогательного переноса (AF): перенос из бита 3 в бит 4
    let af = ((eax & 0x0F) as i32) < ((imm32 & 0x0F) as i32);
    
    // Установка флагов
    machine.registers.set_flags(flags::compute_flags_u32(result, cf, of, af));
    
    machine.log_instruction(csip, &bytes).ok();
}