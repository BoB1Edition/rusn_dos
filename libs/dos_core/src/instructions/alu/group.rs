// Ver: 2 File: ./libs/dos_core/src/instructions/alu/group.rs
use crate::{DosMachine, flags, modrm::ModRm};

pub fn group_x80(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let reg_field = modrm.reg_field;

    if modrm.is_register_mode() {
        let imm8 = machine.read_instr_u8(machine.registers.ip());
        machine.registers.step(None);
        bytes.push(imm8);
        group_x80_operation_register(machine, reg_field, modrm.rm_field, imm8);
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let imm8 = machine.read_instr_u8(machine.registers.ip());
        machine.registers.step(None);
        bytes.push(imm8);
        group_x80_operation_memory(machine, reg_field, addr, imm8);
    }
    machine.log_instruction(csip, &bytes).ok();
}

fn group_x80_operation_register(machine: &mut DosMachine, reg_field: u8, rm_field: u8, imm8: u8) {
    let src_val = machine.read_reg8(rm_field);
    let imm = imm8;
    
    let (result, flags) = match reg_field {
        0 => { // ADD r8, imm8
            let res = src_val as u16 + imm as u16;
            let result = res as u8;
            let cf = (res >> 8) != 0;
            let af = ((src_val & 0x0F) + (imm & 0x0F)) > 0x0F;
            let of = (((src_val ^ imm) & 0x80) == 0) && ((src_val ^ result) & 0x80) != 0;
            (result, flags::compute_flags_u8(machine.registers.flags(), result, cf, of, af))
        }
        1 => { // OR r8, imm8
            let result = src_val | imm;
            (result, flags::compute_logical_flags_u8(machine.registers.flags(), result))
        }
        2 => { // ADC r8, imm8
            let carry_in = (machine.registers.flags() & 1) != 0;
            let mut res = src_val as u16 + imm as u16;
            if carry_in { res += 1; }
            let result = res as u8;
            let cf = (res >> 8) != 0;
            let af = ((src_val & 0x0F) + (imm & 0x0F) + if carry_in { 1 } else { 0 }) > 0x0F;
            let of = (((src_val ^ imm) & 0x80) == 0) && ((src_val ^ result) & 0x80) != 0;
            (result, flags::compute_flags_u8(machine.registers.flags(), result, cf, of, af))
        }
        3 => { // SBB r8, imm8
            let borrow_in = (machine.registers.flags() & 1) != 0;
            let subtrahend = imm as u16 + if borrow_in { 1 } else { 0 };
            let cf = (src_val as u16) < subtrahend;
            let res = (src_val as u16).wrapping_sub(subtrahend);
            let result = res as u8;
            let af = (src_val & 0x0F) < (imm & 0x0F) + if borrow_in { 1 } else { 0 };
            let of = (((src_val ^ imm) & 0x80) != 0) && ((src_val ^ result) & 0x80) != 0;
            (result, flags::compute_flags_u8(machine.registers.flags(), result, cf, of, af))
        }
        4 => { // AND r8, imm8
            let result = src_val & imm;
            (result, flags::compute_logical_flags_u8(machine.registers.flags(), result))
        }
        5 => { // SUB r8, imm8
            let res = (src_val as u16).wrapping_sub(imm as u16);
            let result = res as u8;
            let cf = src_val < imm;
            let af = (src_val & 0x0F) < (imm & 0x0F);
            let of = (((src_val ^ imm) & 0x80) != 0) && ((src_val ^ result) & 0x80) != 0;
            (result, flags::compute_flags_u8(machine.registers.flags(), result, cf, of, af))
        }
        6 => { // XOR r8, imm8
            let result = src_val ^ imm;
            (result, flags::compute_logical_flags_u8(machine.registers.flags(), result))
        }
        7 => { // CMP r8, imm8
            let res = (src_val as u16).wrapping_sub(imm as u16);
            let result = res as u8;
            let cf = src_val < imm;
            let af = (src_val & 0x0F) < (imm & 0x0F);
            let of = (((src_val ^ imm) & 0x80) != 0) && ((src_val ^ result) & 0x80) != 0;
            let flags = flags::compute_flags_u8(machine.registers.flags(), result, cf, of, af);
            (src_val, flags) // результат не сохраняем
        }
        _ => unreachable!(),
    };
    
    // Сохраняем результат только если это не CMP (reg_field != 7)
    if reg_field != 7 {
        machine.write_reg8(rm_field, result);
    }
    machine.registers.set_flags(flags);
}

fn group_x80_operation_memory(machine: &mut DosMachine, reg_field: u8, addr: u32, imm8: u8) {
    let src_val = machine.read_phys_u8(addr);
    let imm = imm8;
    
    let (result, flags) = match reg_field {
        0 => { // ADD [mem], imm8
            let res = src_val as u16 + imm as u16;
            let result = res as u8;
            let cf = (res >> 8) != 0;
            let af = ((src_val & 0x0F) + (imm & 0x0F)) > 0x0F;
            let of = (((src_val ^ imm) & 0x80) == 0) && ((src_val ^ result) & 0x80) != 0;
            (result, flags::compute_flags_u8(machine.registers.flags(), result, cf, of, af))
        }
        1 => { // OR [mem], imm8
            let result = src_val | imm;
            (result, flags::compute_logical_flags_u8(machine.registers.flags(), result))
        }
        2 => { // ADC [mem], imm8
            let carry_in = (machine.registers.flags() & 1) != 0;
            let mut res = src_val as u16 + imm as u16;
            if carry_in { res += 1; }
            let result = res as u8;
            let cf = (res >> 8) != 0;
            let af = ((src_val & 0x0F) + (imm & 0x0F) + if carry_in { 1 } else { 0 }) > 0x0F;
            let of = (((src_val ^ imm) & 0x80) == 0) && ((src_val ^ result) & 0x80) != 0;
            (result, flags::compute_flags_u8(machine.registers.flags(), result, cf, of, af))
        }
        3 => { // SBB [mem], imm8
            let borrow_in = (machine.registers.flags() & 1) != 0;
            let subtrahend = imm as u16 + if borrow_in { 1 } else { 0 };
            let cf = (src_val as u16) < subtrahend;
            let res = (src_val as u16).wrapping_sub(subtrahend);
            let result = res as u8;
            let af = (src_val & 0x0F) < (imm & 0x0F) + if borrow_in { 1 } else { 0 };
            let of = (((src_val ^ imm) & 0x80) != 0) && ((src_val ^ result) & 0x80) != 0;
            (result, flags::compute_flags_u8(machine.registers.flags(), result, cf, of, af))
        }
        4 => { // AND [mem], imm8
            let result = src_val & imm;
            (result, flags::compute_logical_flags_u8(machine.registers.flags(), result))
        }
        5 => { // SUB [mem], imm8
            let res = (src_val as u16).wrapping_sub(imm as u16);
            let result = res as u8;
            let cf = src_val < imm;
            let af = (src_val & 0x0F) < (imm & 0x0F);
            let of = (((src_val ^ imm) & 0x80) != 0) && ((src_val ^ result) & 0x80) != 0;
            (result, flags::compute_flags_u8(machine.registers.flags(), result, cf, of, af))
        }
        6 => { // XOR [mem], imm8
            let result = src_val ^ imm;
            (result, flags::compute_logical_flags_u8(machine.registers.flags(), result))
        }
        7 => { // CMP [mem], imm8
            let res = (src_val as u16).wrapping_sub(imm as u16);
            let result = res as u8;
            let cf = src_val < imm;
            let af = (src_val & 0x0F) < (imm & 0x0F);
            let of = (((src_val ^ imm) & 0x80) != 0) && ((src_val ^ result) & 0x80) != 0;
            let flags = flags::compute_flags_u8(machine.registers.flags(), result, cf, of, af);
            (src_val, flags) // результат не сохраняем
        }
        _ => unreachable!(),
    };
    
    if reg_field != 7 {
        machine.write_phys_u8(addr, result);
    }
    machine.registers.set_flags(flags);
}

pub fn group_x83_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        let imm8 = machine.read_instr_u8(machine.registers.ip());
        machine.registers.step(None);
        bytes.push(imm8);
        let imm16 = (imm8 as i8) as i16 as u16;
        let dst_val = machine.read_reg16(modrm.rm_field);
        let (result, flags) = perform_group_x83_operation_16(machine, modrm.reg_field, dst_val, imm16, machine.registers.flags());
        if modrm.reg_field != 7 { machine.write_reg16(modrm.rm_field, result); }
        machine.registers.set_flags(flags);
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let imm8 = machine.read_instr_u8(machine.registers.ip());
        machine.registers.step(None);
        bytes.push(imm8);
        let imm16 = (imm8 as i8) as i16 as u16;
        let dst_val = machine.read_phys_u16(addr);
        let (result, flags) = perform_group_x83_operation_16(machine, modrm.reg_field, dst_val, imm16, machine.registers.flags());
        if modrm.reg_field != 7 { machine.write_phys_u16(addr, result); }
        machine.registers.set_flags(flags);
    }
    machine.log_instruction(csip, &bytes).ok();
}

fn perform_group_x83_operation_16(machine: &DosMachine, op_field: u8, dst_val: u16, imm: u16, flags: u16) -> (u16, u16) {
    match op_field {
        0 => { // ADD r/m16, imm8 (sign-extended)
            let res = dst_val as u32 + imm as u32;
            let result = res as u16;
            let cf = res > 0xFFFF;
            let af = ((dst_val & 0x0F) + (imm & 0x0F)) > 0x0F;
            let of = (((dst_val ^ imm) & 0x8000) == 0) && ((dst_val ^ result) & 0x8000) != 0;
            (result, flags::compute_flags_u16(machine.registers.flags(), result, cf, of, af))
        }
        1 => { // OR r/m16, imm8
            let result = dst_val | imm;
            (result, flags::compute_logical_flags_u16(machine.registers.flags(), result))
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
            (result, flags::compute_flags_u16(machine.registers.flags(), result, cf, of, af))
        }
        3 => { // SBB r/m16, imm8
            let borrow_in = (flags & 1) != 0;
            let imm_with_borrow = imm as u32 + if borrow_in { 1 } else { 0 };
            let cf = (dst_val as u32) < imm_with_borrow;
            let res = (dst_val as u32).wrapping_sub(imm_with_borrow);
            let result = res as u16;
            let af = (dst_val & 0x0F) < ((imm & 0x0F) + if borrow_in { 1 } else { 0 });
            let of = (((dst_val ^ imm) & 0x8000) != 0) && (((dst_val ^ result) & 0x8000) != 0);
            (result, flags::compute_flags_u16(machine.registers.flags(), result, cf, of, af))
        }
        4 => { // AND r/m16, imm8
            let result = dst_val & imm;
            (result, flags::compute_logical_flags_u16(machine.registers.flags(), result))
        }
        5 => { // SUB r/m16, imm8
            let res = (dst_val as u32).wrapping_sub(imm as u32);
            let result = res as u16;
            let cf = dst_val < imm;
            let af = (dst_val & 0x0F) < (imm & 0x0F);
            let of = (((dst_val ^ imm) & 0x8000) != 0) && (((dst_val ^ result) & 0x8000) != 0);
            (result, flags::compute_flags_u16(machine.registers.flags(), result, cf, of, af))
        }
        6 => { // XOR r/m16, imm8
            let result = dst_val ^ imm;
            (result, flags::compute_logical_flags_u16(machine.registers.flags(), result))
        }
        7 => { // CMP r/m16, imm8 — как SUB, но не сохраняем результат
            let res = (dst_val as u32).wrapping_sub(imm as u32);
            let result = res as u16;
            let cf = dst_val < imm;
            let af = (dst_val & 0x0F) < (imm & 0x0F);
            let of = (((dst_val ^ imm) & 0x8000) != 0) && (((dst_val ^ result) & 0x8000) != 0);
            let flags = flags::compute_flags_u16(machine.registers.flags(), result, cf, of, af);
            (dst_val, flags) // Возвращаем исходное значение (не сохраняем результат)
        }
        _ => unreachable!(),
    }
}

pub fn group_fe_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
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
                (result, flags::compute_flags_u8(machine.registers.flags(), result, current_cf, of, af))
            }
            1 => { // DEC r8
                let result = value.wrapping_sub(1);
                let af = (value & 0x0F) == 0;
                let of = value == 0x80; // 10000000 → 01111111 (знаковое переполнение)
                (result, flags::compute_flags_u8(machine.registers.flags(), result, current_cf, of, af))
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
                (result, flags::compute_flags_u8(machine.registers.flags(), result, current_cf, of, af))
            }
            1 => { // DEC [mem]
                let result = value.wrapping_sub(1);
                let af = (value & 0x0F) == 0;
                let of = value == 0x80;
                (result, flags::compute_flags_u8(machine.registers.flags(), result, current_cf, of, af))
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

pub fn group_f6_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let reg_field = modrm.reg_field;

    if modrm.is_register_mode() {
        let imm8 = if reg_field == 0 || reg_field == 1 {
            let imm = machine.read_instr_u8(machine.registers.ip());
            machine.registers.step(None);
            bytes.push(imm);
            Some(imm)
        } else { None };
        group_f6_register(machine, reg_field, modrm.rm_field, imm8);
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let imm8 = if reg_field == 0 || reg_field == 1 {
            let imm = machine.read_instr_u8(machine.registers.ip());
            machine.registers.step(None);
            bytes.push(imm);
            Some(imm)
        } else { None };
        group_f6_memory(machine, reg_field, addr, imm8);
    }
    machine.log_instruction(csip, &bytes).ok();
}

// Вспомогательная функция для операций над регистром
fn group_f6_register(machine: &mut DosMachine, reg_field: u8, rm_field: u8, imm8: Option<u8>) {
    let value = machine.read_reg8(rm_field);
    
    match reg_field {
        0 | 1 => {
            // TEST r8, imm8
            let imm = imm8.unwrap();
            let result = value & imm;
            machine.registers.set_flags(flags::compute_logical_flags_u8(machine.registers.flags(), result));
        }
        2 => {
            // NOT r8
            let result = !value;
            machine.write_reg8(rm_field, result);
            // Флаги НЕ изменяются!
        }
        3 => {
            // NEG r8
            let result = value.wrapping_neg();
            let cf = value != 0; // CF=1 если исходное значение ≠ 0
            let af = (value & 0x0F) != 0;
            let of = value == 0x80; // -128 → 128 (знаковое переполнение)
            machine.write_reg8(rm_field, result);
            machine.registers.set_flags(flags::compute_flags_u8(machine.registers.flags(), result, cf, of, af));
        }
        4 => {
            // MUL r8 (беззнаковое)
            let src = value as u16;
            let al = machine.registers.al() as u16;
            let product = al * src;
            machine.registers.set_ax(product);
            let cf_of = product > 0xFF;
            let mut flags = machine.registers.flags();
            if cf_of {
                flags |= flags::CF;  // CF = 1
                flags |= flags::OF; // OF = 1
            } else {
                flags &= !(flags::CF | flags::OF); // CF = OF = 0
            }
            // Остальные флаги не определены (обычно не изменяются)
            machine.registers.set_flags(flags);
        }
        5 => {
            // IMUL r8 (знаковое)
            let src = value as i8 as i16;
            let al = machine.registers.al() as i8 as i16;
            let product = al * src;
            machine.registers.set_ax(product as u16);
            let cf_of = product < -128 || product > 127;
            let mut flags = machine.registers.flags();
            if cf_of {
                flags |= flags::CF;  // CF = 1
                flags |= flags::OF; // OF = 1
            } else {
                flags &= !(flags::CF | flags::OF); // CF = OF = 0
            }
            machine.registers.set_flags(flags);
        }
        6 => {
            // DIV r8 (беззнаковое)
            let divisor = value;
            if divisor == 0 {
                log::error!("Division by zero in DIV r8");
                machine.halted = true;
                return;
            }
            let dividend = machine.registers.ax() as u16;
            let quotient = dividend / divisor as u16;
            let remainder = dividend % divisor as u16;
            if quotient > 0xFF {
                log::error!("Divide overflow in DIV r8 (quotient > 0xFF)");
                machine.halted = true;
                return;
            }
            machine.registers.set_al(quotient as u8);
            machine.registers.set_ah(remainder as u8);
            // Флаги не определены (обычно не изменяются)
        }
        7 => {
            // IDIV r8 (знаковое)
            let divisor = value as i8;
            if divisor == 0 {
                log::error!("Division by zero in IDIV r8");
                machine.halted = true;
                return;
            }
            let dividend = machine.registers.ax() as i16;
            // Особый случай: -32768 / -1 вызывает переполнение
            if dividend == -32768 && divisor == -1 {
                log::error!("Divide overflow in IDIV r8 (-32768 / -1)");
                machine.halted = true;
                return;
            }
            let quotient = dividend / divisor as i16;
            let remainder = dividend % divisor as i16;
            if quotient < -128 || quotient > 127 {
                log::error!("Divide overflow in IDIV r8 (quotient out of range)");
                machine.halted = true;
                return;
            }
            machine.registers.set_al(quotient as u8);
            machine.registers.set_ah(remainder as u8);
            // Флаги не определены (обычно не изменяются)
        }
        _ => unreachable!(),
    }
}

// Вспомогательная функция для операций над памятью
fn group_f6_memory(machine: &mut DosMachine, reg_field: u8, addr: u32, imm8: Option<u8>) {
    let value = machine.read_phys_u8(addr);
    
    match reg_field {
        0 | 1 => {
            // TEST [mem], imm8
            let imm = imm8.unwrap();
            let result = value & imm;
            machine.registers.set_flags(flags::compute_logical_flags_u8(machine.registers.flags(), result));
        }
        2 => {
            // NOT [mem]
            let result = !value;
            machine.write_phys_u8(addr, result);
            // Флаги НЕ изменяются!
        }
        3 => {
            // NEG [mem]
            let result = value.wrapping_neg();
            let cf = value != 0;
            let af = (value & 0x0F) != 0;
            let of = value == 0x80;
            machine.write_phys_u8(addr, result);
            machine.registers.set_flags(flags::compute_flags_u8(machine.registers.flags(), result, cf, of, af));
        }
        4 => {
            // MUL [mem] (беззнаковое)
            let src = value as u16;
            let al = machine.registers.al() as u16;
            let product = al * src;
            machine.registers.set_ax(product);
            let cf_of = product > 0xFF;
            let mut flags = machine.registers.flags();
            if cf_of {
                flags |= flags::CF | flags::OF;
            } else {
                flags &= !(flags::CF | flags::OF);
            }
            machine.registers.set_flags(flags);
        }
        5 => {
            // IMUL [mem] (знаковое)
            let src = value as i8 as i16;
            let al = machine.registers.al() as i8 as i16;
            let product = al * src;
            machine.registers.set_ax(product as u16);
            let cf_of = product < -128 || product > 127;
            let mut flags = machine.registers.flags();
            if cf_of {
                flags |= flags::CF | flags::OF;
            } else {
                flags &= !(flags::CF | flags::OF);
            }
            machine.registers.set_flags(flags);
        }
        6 => {
            // DIV [mem] (беззнаковое)
            let divisor = value;
            if divisor == 0 {
                log::error!("Division by zero in DIV [mem]");
                machine.halted = true;
                return;
            }
            let dividend = machine.registers.ax() as u16;
            let quotient = dividend / divisor as u16;
            let remainder = dividend % divisor as u16;
            if quotient > 0xFF {
                log::error!("Divide overflow in DIV [mem]");
                machine.halted = true;
                return;
            }
            machine.registers.set_al(quotient as u8);
            machine.registers.set_ah(remainder as u8);
        }
        7 => {
            // IDIV [mem] (знаковое)
            let divisor = value as i8;
            if divisor == 0 {
                log::error!("Division by zero in IDIV [mem]");
                machine.halted = true;
                return;
            }
            let dividend = machine.registers.ax() as i16;
            if dividend == -32768 && divisor == -1 {
                log::error!("Divide overflow in IDIV [mem] (-32768 / -1)");
                machine.halted = true;
                return;
            }
            let quotient = dividend / divisor as i16;
            let remainder = dividend % divisor as i16;
            if quotient < -128 || quotient > 127 {
                log::error!("Divide overflow in IDIV [mem]");
                machine.halted = true;
                return;
            }
            machine.registers.set_al(quotient as u8);
            machine.registers.set_ah(remainder as u8);
        }
        _ => unreachable!(),
    }
}

pub fn group_f7_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let reg_field = modrm.reg_field;

    if modrm.is_register_mode() {
        let imm16 = if reg_field == 0 || reg_field == 1 {
            let imm = machine.read_instr_u16(machine.registers.ip());
            machine.registers.step(Some(2));
            bytes.extend_from_slice(&imm.to_le_bytes());
            Some(imm)
        } else { None };
        group_f7_register(machine, reg_field, modrm.rm_field, imm16);
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let imm16 = if reg_field == 0 || reg_field == 1 {
            let imm = machine.read_instr_u16(machine.registers.ip());
            machine.registers.step(Some(2));
            bytes.extend_from_slice(&imm.to_le_bytes());
            Some(imm)
        } else { None };
        group_f7_memory(machine, reg_field, addr, imm16);
    }
    machine.log_instruction(csip, &bytes).ok();
}

// Вспомогательная функция для операций над регистром
fn group_f7_register(machine: &mut DosMachine, reg_field: u8, rm_field: u8, imm16: Option<u16>) {
    let value = machine.read_reg16(rm_field);
    
    match reg_field {
        0 | 1 => {
            // TEST r16, imm16
            let imm = imm16.unwrap();
            let result = value & imm;
            machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
        }
        2 => {
            // NOT r16
            let result = !value;
            machine.write_reg16(rm_field, result);
            // Флаги НЕ изменяются!
        }
        3 => {
            // NEG r16
            let result = value.wrapping_neg();
            let cf = value != 0;
            let af = (value & 0x0F) != 0;
            let of = value == 0x8000; // -32768 → 32768 (знаковое переполнение)
            machine.write_reg16(rm_field, result);
            machine.registers.set_flags(flags::compute_flags_u16(machine.registers.flags(), result, cf, of, af));
        }
        4 => {
            // MUL r16 (беззнаковое 16×16→32)
            let src = value as u32;
            let ax = machine.registers.ax() as u32;
            let product = ax * src;
            machine.registers.set_ax((product & 0xFFFF) as u16);
            machine.registers.set_dx((product >> 16) as u16);
            let cf_of = product > 0xFFFF;
            let mut flags = machine.registers.flags();
            if cf_of {
                flags |= flags::CF;  // CF = 1
                flags |= flags::OF; // OF = 1
            } else {
                flags &= !(flags::CF | flags::OF); // CF = OF = 0
            }
            machine.registers.set_flags(flags);
        }
        5 => {
            // IMUL r16 (знаковое 16×16→32)
            let src = value as i16 as i32;
            let ax = machine.registers.ax() as i16 as i32;
            let product = ax * src;
            machine.registers.set_ax((product & 0xFFFF) as u16);
            machine.registers.set_dx(((product >> 16) & 0xFFFF) as u16);
            let cf_of = product < -32768 || product > 32767;
            let mut flags = machine.registers.flags();
            if cf_of {
                flags |= flags::CF;  // CF = 1
                flags |= flags::OF; // OF = 1
            } else {
                flags &= !(flags::CF | flags::OF); // CF = OF = 0
            }
            machine.registers.set_flags(flags);
        }
        6 => {
            // DIV r16 (беззнаковое 32/16→16+16)
            let divisor = value;
            if divisor == 0 {
                log::error!("Division by zero in DIV r16");
                machine.halted = true;
                return;
            }
            let dividend = ((machine.registers.dx() as u32) << 16) | (machine.registers.ax() as u32);
            let quotient = dividend / divisor as u32;
            let remainder = dividend % divisor as u32;
            if quotient > 0xFFFF {
                log::error!("Divide overflow in DIV r16 (quotient > 0xFFFF)");
                machine.halted = true;
                return;
            }
            machine.registers.set_ax(quotient as u16);
            machine.registers.set_dx(remainder as u16);
            // Флаги не определены (обычно не изменяются)
        }
        7 => {
            // IDIV r16 (знаковое 32/16→16+16)
            let divisor = value as i16;
            if divisor == 0 {
                log::error!("Division by zero in IDIV r16");
                machine.halted = true;
                return;
            }
            let dividend = ((machine.registers.dx() as i32) << 16) | (machine.registers.ax() as i32);
            // Особый случай: -2147483648 / -1 вызывает переполнение
            if dividend == -2147483648 && divisor == -1 {
                log::error!("Divide overflow in IDIV r16 (-2147483648 / -1)");
                machine.halted = true;
                return;
            }
            let quotient = dividend / divisor as i32;
            let remainder = dividend % divisor as i32;
            if quotient < -32768 || quotient > 32767 {
                log::error!("Divide overflow in IDIV r16 (quotient out of range)");
                machine.halted = true;
                return;
            }
            machine.registers.set_ax(quotient as u16);
            machine.registers.set_dx(remainder as u16);
            // Флаги не определены (обычно не изменяются)
        }
        _ => unreachable!(),
    }
}

// Вспомогательная функция для операций над памятью
fn group_f7_memory(machine: &mut DosMachine, reg_field: u8, addr: u32, imm16: Option<u16>) {
    let value = machine.read_phys_u16(addr);
    
    match reg_field {
        0 | 1 => {
            // TEST [mem], imm16
            let imm = imm16.unwrap();
            let result = value & imm;
            machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
        }
        2 => {
            // NOT [mem]
            let result = !value;
            machine.write_phys_u16(addr, result);
            // Флаги НЕ изменяются!
        }
        3 => {
            // NEG [mem]
            let result = value.wrapping_neg();
            let cf = value != 0;
            let af = (value & 0x0F) != 0;
            let of = value == 0x8000;
            machine.write_phys_u16(addr, result);
            machine.registers.set_flags(flags::compute_flags_u16(machine.registers.flags(), result, cf, of, af));
        }
        4 => {
            // MUL [mem] (беззнаковое)
            let src = value as u32;
            let ax = machine.registers.ax() as u32;
            let product = ax * src;
            machine.registers.set_ax((product & 0xFFFF) as u16);
            machine.registers.set_dx((product >> 16) as u16);
            let cf_of = product > 0xFFFF;
            let mut flags = machine.registers.flags();
            if cf_of {
                flags |= flags::CF | flags::OF;
            } else {
                flags &= !(flags::CF | flags::OF);
            }
            machine.registers.set_flags(flags);
        }
        5 => {
            // IMUL [mem] (знаковое)
            let src = value as i16 as i32;
            let ax = machine.registers.ax() as i16 as i32;
            let product = ax * src;
            machine.registers.set_ax((product & 0xFFFF) as u16);
            machine.registers.set_dx(((product >> 16) & 0xFFFF) as u16);
            let cf_of = product < -32768 || product > 32767;
            let mut flags = machine.registers.flags();
            if cf_of {
                flags |= flags::CF | flags::OF;
            } else {
                flags &= !(flags::CF | flags::OF);
            }
            machine.registers.set_flags(flags);
        }
        6 => {
            // DIV [mem] (беззнаковое)
            let divisor = value;
            if divisor == 0 {
                log::error!("Division by zero in DIV [mem]");
                machine.halted = true;
                return;
            }
            let dividend = ((machine.registers.dx() as u32) << 16) | (machine.registers.ax() as u32);
            let quotient = dividend / divisor as u32;
            let remainder = dividend % divisor as u32;
            if quotient > 0xFFFF {
                log::error!("Divide overflow in DIV [mem]");
                machine.halted = true;
                return;
            }
            machine.registers.set_ax(quotient as u16);
            machine.registers.set_dx(remainder as u16);
        }
        7 => {
            // IDIV [mem] (знаковое)
            let divisor = value as i16;
            if divisor == 0 {
                log::error!("Division by zero in IDIV [mem]");
                machine.halted = true;
                return;
            }
            let dividend = ((machine.registers.dx() as i32) << 16) | (machine.registers.ax() as i32);
            if dividend == -2147483648 && divisor == -1 {
                log::error!("Divide overflow in IDIV [mem] (-2147483648 / -1)");
                machine.halted = true;
                return;
            }
            let quotient = dividend / divisor as i32;
            let remainder = dividend % divisor as i32;
            if quotient < -32768 || quotient > 32767 {
                log::error!("Divide overflow in IDIV [mem]");
                machine.halted = true;
                return;
            }
            machine.registers.set_ax(quotient as u16);
            machine.registers.set_dx(remainder as u16);
        }
        _ => unreachable!(),
    }
}

pub fn group_x81_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        let imm16 = machine.read_instr_u16(machine.registers.ip());
        machine.registers.step(Some(2));
        bytes.extend_from_slice(&imm16.to_le_bytes());
        let dst_val = machine.read_reg16(modrm.rm_field);
        let (result, flags) = perform_group_x83_operation_16(machine, modrm.reg_field, dst_val, imm16, machine.registers.flags());
        if modrm.reg_field != 7 { machine.write_reg16(modrm.rm_field, result); }
        machine.registers.set_flags(flags);
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let imm16 = machine.read_instr_u16(machine.registers.ip());
        machine.registers.step(Some(2));
        bytes.extend_from_slice(&imm16.to_le_bytes());
        let dst_val = machine.read_phys_u16(addr);
        let (result, flags) = perform_group_x83_operation_16(machine, modrm.reg_field, dst_val, imm16, machine.registers.flags());
        if modrm.reg_field != 7 { machine.write_phys_u16(addr, result); }
        machine.registers.set_flags(flags);
    }
    machine.log_instruction(csip, &bytes).ok();
}