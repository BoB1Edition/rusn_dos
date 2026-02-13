use log::error;

use crate::{DosMachine, flags, modrm::ModRm};

pub fn group_x80(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    if !machine.has_address_size_prefix {
        let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
        let imm8 = machine.read_u8(
            machine.registers.cs(),
            machine.registers.ip().wrapping_add(1),
        );
        machine.registers.step(Some(2));

        bytes.push(modrm_byte);
        bytes.push(imm8);
        let modrm = ModRm::from_byte(modrm_byte);
        if modrm.is_register_mode() {
            group_x80_operation_registry(machine, modrm.reg_field, modrm.rm_field, imm8);
        } else {
            error!("Memory operand in group_x80 not supported yet");
            machine.halted = true;
        }
    } else {
        machine.print_error_exit(bytes.last().unwrap().clone());
    }
     machine.log_instruction(csip, &bytes).ok();
}

fn group_x80_operation_registry(machine: &mut DosMachine, reg_field: u8, rm_field: u8, imm8: u8) {
    let src_val = machine.read_reg8(rm_field); // ← вместо get_registry_value
    let imm = imm8 as u8;

    // Все вычисления делаем в u16, чтобы ловить переносы
    let (result_u8, flags) = match reg_field {
        0 => {
            // ADD r8, imm8
            let res = src_val as u16 + imm as u16;
            let result = res as u8;
            let cf = (res >> 8) != 0;
            let af = ((src_val & 0x0F) + (imm & 0x0F)) > 0x0F;
            let of = (((src_val ^ imm) & 0x80) == 0) && ((src_val ^ result) & 0x80) != 0;
            (result, flags::compute_flags_u8(result, cf, of, af))
        }
        1 => {
            // OR r8, imm8
            let result = src_val | imm;
            (result, flags::compute_logical_flags_u8(result))
        }
        2 => {
            // ADC r8, imm8
            let carry_in = (machine.registers.flags() & 1) != 0;
            let mut res = src_val as u16 + imm as u16;
            if carry_in {
                res += 1;
            }
            let result = res as u8;
            let cf = (res >> 8) != 0;
            let af = ((src_val & 0x0F) + (imm & 0x0F) + if carry_in { 1 } else { 0 }) > 0x0F;
            let of = (((src_val ^ imm) & 0x80) == 0) && ((src_val ^ result) & 0x80) != 0;
            (result, flags::compute_flags_u8(result, cf, of, af))
        }
        3 => {
            // SBB r8, imm8
            let borrow_in = (machine.registers.flags() & 1) != 0;
            let mut res = src_val as u16;
            let subtrahend = imm as u16 + if borrow_in { 1 } else { 0 };
            let cf = res < subtrahend;
            res = res.wrapping_sub(subtrahend);
            let result = res as u8;
            let af = (src_val & 0x0F) < (imm & 0x0F) + if borrow_in { 1 } else { 0 };
            let of = (((src_val ^ imm) & 0x80) != 0) && ((src_val ^ result) & 0x80) != 0;
            (result, flags::compute_flags_u8(result, cf, of, af))
        }
        4 => {
            // AND r8, imm8
            let result = src_val & imm;
            (result, flags::compute_logical_flags_u8(result))
        }
        5 => {
            // SUB r8, imm8
            let res = (src_val as u16).wrapping_sub(imm as u16);
            let result = res as u8;
            let cf = src_val < imm;
            let af = (src_val & 0x0F) < (imm & 0x0F);
            let of = (((src_val ^ imm) & 0x80) != 0) && ((src_val ^ result) & 0x80) != 0;
            (result, flags::compute_flags_u8(result, cf, of, af))
        }
        6 => {
            // XOR r8, imm8
            let result = src_val ^ imm;
            (result, flags::compute_logical_flags_u8(result))
        }
        7 => {
            // CMP r8, imm8 — как SUB, но не сохраняем результат
            let res = (src_val as u16).wrapping_sub(imm as u16);
            let result = res as u8;
            let cf = src_val < imm;
            let af = (src_val & 0x0F) < (imm & 0x0F);
            let of = (((src_val ^ imm) & 0x80) != 0) && ((src_val ^ result) & 0x80) != 0;
            let flags = flags::compute_flags_u8(result, cf, of, af);
            machine.registers.set_flags(flags);
            return; // выход без записи результата
        }
        _ => unreachable!(),
    };

    // Записываем результат (если не CMP)
    if reg_field != 7 {
        machine.write_reg8(rm_field, result_u8);
    }

    machine.registers.set_flags(flags);
}

pub fn group_x83_rm16(machine: &mut DosMachine, prev: &[u8]) {
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
    
    // Расширяем imm8 до i8, затем до i16 со знаком (sign-extend)
    let imm16 = (imm8 as i8) as i16 as u16;
    
    if modrm.is_register_mode() {
        // Операция над регистром
        let dst_val = machine.read_reg16(modrm.rm_field);
        let (result, flags) = perform_group_x83_operation_16(modrm.reg_field, dst_val, imm16, machine.registers.flags());
        if modrm.reg_field != 7 { // CMP (reg_field=7) не сохраняет результат
            machine.write_reg16(modrm.rm_field, result);
        }
        machine.registers.set_flags(flags);
    } else {
        // Операция над памятью
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let dst_val = machine.read_phys_u16(addr);
        let (result, flags) = perform_group_x83_operation_16(modrm.reg_field, dst_val, imm16, machine.registers.flags());
        if modrm.reg_field != 7 { // CMP не сохраняет результат
            machine.write_phys_u16(addr, result);
        }
        machine.registers.set_flags(flags);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

fn perform_group_x83_operation_16(op_field: u8, dst_val: u16, imm: u16, flags: u16) -> (u16, u16) {
    match op_field {
        0 => { // ADD r/m16, imm8 (sign-extended)
            let res = dst_val as u32 + imm as u32;
            let result = res as u16;
            let cf = res > 0xFFFF;
            let af = ((dst_val & 0x0F) + (imm & 0x0F)) > 0x0F;
            let of = (((dst_val ^ imm) & 0x8000) == 0) && ((dst_val ^ result) & 0x8000) != 0;
            (result, flags::compute_flags_u16(result, cf, of, af))
        }
        1 => { // OR r/m16, imm8
            let result = dst_val | imm;
            (result, flags::compute_logical_flags_u16(result))
        }
        2 => { // ADC r/m16, imm8
            let carry_in = (flags & 1) != 0;
            let mut res = dst_val as u32 + imm as u32;
            if carry_in {
                res += 1;
            }
            let result = res as u16;
            let cf = res > 0xFFFF;
            let af = ((dst_val & 0x0F) + (imm & 0x0F) + if carry_in { 1 } else { 0 }) > 0x0F;
            let of = (((dst_val ^ imm) & 0x8000) == 0) && ((dst_val ^ result) & 0x8000) != 0;
            (result, flags::compute_flags_u16(result, cf, of, af))
        }
        3 => { // SBB r/m16, imm8
            let borrow_in = (flags & 1) != 0;
            let imm_with_borrow = imm as u32 + if borrow_in { 1 } else { 0 };
            let cf = (dst_val as u32) < imm_with_borrow;
            let res = (dst_val as u32).wrapping_sub(imm_with_borrow);
            let result = res as u16;
            let af = (dst_val & 0x0F) < ((imm & 0x0F) + if borrow_in { 1 } else { 0 });
            let of = (((dst_val ^ imm) & 0x8000) != 0) && (((dst_val ^ result) & 0x8000) != 0);
            (result, flags::compute_flags_u16(result, cf, of, af))
        }
        4 => { // AND r/m16, imm8
            let result = dst_val & imm;
            (result, flags::compute_logical_flags_u16(result))
        }
        5 => { // SUB r/m16, imm8
            let res = (dst_val as u32).wrapping_sub(imm as u32);
            let result = res as u16;
            let cf = dst_val < imm;
            let af = (dst_val & 0x0F) < (imm & 0x0F);
            let of = (((dst_val ^ imm) & 0x8000) != 0) && (((dst_val ^ result) & 0x8000) != 0);
            (result, flags::compute_flags_u16(result, cf, of, af))
        }
        6 => { // XOR r/m16, imm8
            let result = dst_val ^ imm;
            (result, flags::compute_logical_flags_u16(result))
        }
        7 => { // CMP r/m16, imm8 — как SUB, но не сохраняем результат
            let res = (dst_val as u32).wrapping_sub(imm as u32);
            let result = res as u16;
            let cf = dst_val < imm;
            let af = (dst_val & 0x0F) < (imm & 0x0F);
            let of = (((dst_val ^ imm) & 0x8000) != 0) && (((dst_val ^ result) & 0x8000) != 0);
            let flags = flags::compute_flags_u16(result, cf, of, af);
            (dst_val, flags) // Возвращаем исходное значение (не сохраняем результат)
        }
        _ => unreachable!(),
    }
}

pub fn group_fe_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Сохраняем текущий флаг переноса (CF) — он НЕ изменяется для INC/DEC
    let current_cf = (machine.registers.flags() & 1) != 0;
    
    if modrm.is_register_mode() {
        // Операция над регистром
        let value = machine.read_reg8(modrm.rm_field);
        let (result, new_flags) = match modrm.reg_field {
            0 => { // INC r8
                let result = value.wrapping_add(1);
                let af = ((value & 0x0F) + 1) > 0x0F;
                let of = value == 0x7F; // 01111111 → 10000000 (знаковое переполнение)
                (result, flags::compute_flags_u8(result, current_cf, of, af))
            }
            1 => { // DEC r8
                let result = value.wrapping_sub(1);
                let af = (value & 0x0F) == 0;
                let of = value == 0x80; // 10000000 → 01111111 (знаковое переполнение)
                (result, flags::compute_flags_u8(result, current_cf, of, af))
            }
            _ => {
                log::error!("Unsupported reg_field {} in group FE", modrm.reg_field);
                machine.halted = true;
                machine.log_instruction(csip, &bytes).ok();
                return;
            }
        };
        machine.write_reg8(modrm.rm_field, result);
        machine.registers.set_flags(new_flags);
    } else {
        // Операция над памятью
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let value = machine.read_phys_u8(addr);
        let (result, new_flags) = match modrm.reg_field {
            0 => { // INC [mem]
                let result = value.wrapping_add(1);
                let af = ((value & 0x0F) + 1) > 0x0F;
                let of = value == 0x7F;
                (result, flags::compute_flags_u8(result, current_cf, of, af))
            }
            1 => { // DEC [mem]
                let result = value.wrapping_sub(1);
                let af = (value & 0x0F) == 0;
                let of = value == 0x80;
                (result, flags::compute_flags_u8(result, current_cf, of, af))
            }
            _ => {
                log::error!("Unsupported reg_field {} in group FE", modrm.reg_field);
                machine.halted = true;
                machine.log_instruction(csip, &bytes).ok();
                return;
            }
        };
        machine.write_phys_u8(addr, result);
        machine.registers.set_flags(new_flags);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}