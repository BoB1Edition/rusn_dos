// Ver: 3
use log::error;

use crate::{machine::DosMachine, modrm::ModRm};

pub fn xor(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg8(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u8(addr)
    };
    let dst_val = machine.read_reg8(modrm.reg_field);
    let result = dst_val ^ src_val;

    machine.write_reg8(modrm.reg_field, result);
    machine
        .registers
        .set_flags(DosMachine::compute_logical_flags_u8(result));
    machine.log_instruction(csip, &bytes).ok();
}
pub fn add_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg8(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u8(addr)
    };
    let dst_val = machine.read_reg8(modrm.rm_field); // приёмник
    let res = (dst_val as u16) + (src_val as u16);
    let result = res as u8;

    let cf = res > 0xFF;
    let af = ((dst_val & 0x0F) + (src_val & 0x0F)) > 0x0F;
    let of = (((dst_val ^ src_val) & 0x80) == 0) && ((dst_val ^ result) & 0x80) != 0;

    machine.write_reg8(modrm.rm_field, result);
    machine
        .registers
        .set_flags(DosMachine::compute_flags_u8(result, cf, of, af));
    machine.log_instruction(csip, &bytes).ok();
}

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
            (result, DosMachine::compute_flags_u8(result, cf, of, af))
        }
        1 => {
            // OR r8, imm8
            let result = src_val | imm;
            (result, DosMachine::compute_logical_flags_u8(result))
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
            (result, DosMachine::compute_flags_u8(result, cf, of, af))
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
            (result, DosMachine::compute_flags_u8(result, cf, of, af))
        }
        4 => {
            // AND r8, imm8
            let result = src_val & imm;
            (result, DosMachine::compute_logical_flags_u8(result))
        }
        5 => {
            // SUB r8, imm8
            let res = (src_val as u16).wrapping_sub(imm as u16);
            let result = res as u8;
            let cf = src_val < imm;
            let af = (src_val & 0x0F) < (imm & 0x0F);
            let of = (((src_val ^ imm) & 0x80) != 0) && ((src_val ^ result) & 0x80) != 0;
            (result, DosMachine::compute_flags_u8(result, cf, of, af))
        }
        6 => {
            // XOR r8, imm8
            let result = src_val ^ imm;
            (result, DosMachine::compute_logical_flags_u8(result))
        }
        7 => {
            // CMP r8, imm8 — как SUB, но не сохраняем результат
            let res = (src_val as u16).wrapping_sub(imm as u16);
            let result = res as u8;
            let cf = src_val < imm;
            let af = (src_val & 0x0F) < (imm & 0x0F);
            let of = (((src_val ^ imm) & 0x80) != 0) && ((src_val ^ result) & 0x80) != 0;
            let flags = DosMachine::compute_flags_u8(result, cf, of, af);
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

pub fn group_x80_operation_memory(machine: &mut DosMachine, reg_field: u8, rm_field: u8, imm8: u8) {
    todo!("group_x80_operation_memory")
}

pub fn group_x80_operation_memory_1byte(
    machine: &mut DosMachine,
    reg_field: u8,
    rm_field: u8,
    imm8: u8,
) {
    todo!("group_x80_operation_memory_1byte")
}

pub fn group_x80_operation_memory_2byte(
    machine: &mut DosMachine,
    reg_field: u8,
    rm_field: u8,
    imm8: u8,
) {
    todo!("group_x80_operation_memory_2byte")
}

pub fn add_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];

    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u16(addr)
    };
    do_add_16(machine, modrm.rm_field, src_val);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn sub_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    machine.log_instruction(csip, &bytes).ok();

    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        // SUB reg16, reg16
        let dst_val = machine.read_reg16(modrm.reg_field); // приёмник
        let src_val = machine.read_reg16(modrm.rm_field); // источник
        let res = (dst_val as i32) - (src_val as i32);
        let result = res as u16;
        let cf = (dst_val as u32) < (src_val as u32);
        let af = (dst_val & 0x0F) < (src_val & 0x0F);
        let of = (((dst_val ^ src_val) & 0x8000) != 0) && (((dst_val ^ result) & 0x8000) != 0);
        machine.write_reg16(modrm.reg_field, result);
        machine
            .registers
            .set_flags(DosMachine::compute_flags_u16(result, cf, of, af));
    } else {
        log::error!("Memory operand in SUB r16, r/m16 not supported yet");
        machine.halted = true;
    }
}

pub fn add_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];

    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u16(addr)
    };
    do_add_16(machine, modrm.reg_field, src_val);
    machine.log_instruction(csip, &bytes).ok();
}

// Внутренняя функция — не вызывается напрямую из execute()
fn do_add_16(machine: &mut DosMachine, dst_reg: u8, src_value: u16) {
    let dst_val = machine.read_reg16(dst_reg);
    let res = dst_val as u32 + src_value as u32;
    let result = res as u16;
    let cf = res > 0xFFFF;
    let af = ((dst_val & 0x0F) + (src_value & 0x0F)) > 0x0F;
    let of = (((dst_val ^ src_value) & 0x8000) == 0) && ((dst_val ^ result) & 0x8000) != 0;

    machine.write_reg16(dst_reg, result);
    machine
        .registers
        .set_flags(DosMachine::compute_flags_u16(result, cf, of, af));
}

pub fn cmp_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u16(addr)
    };

    let dst_val = machine.read_reg16(modrm.reg_field);
    let res = (dst_val as i32) - (src_val as i32);
    let result = res as u16;
    let cf = (dst_val as u32) < (src_val as u32);
    let af = (dst_val & 0x0F) < (src_val & 0x0F);
    let of = (((dst_val ^ src_val) & 0x8000) != 0) && (((dst_val ^ result) & 0x8000) != 0);

    machine.registers.set_flags(DosMachine::compute_flags_u16(result, cf, of, af));
    machine.log_instruction(csip, &bytes).ok();
}

pub fn xchg_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    if modrm.is_register_mode() {
        // XCHG reg16, reg16
        let src_val = machine.read_reg16(modrm.reg_field);
        let dst_val = machine.read_reg16(modrm.rm_field);
        machine.write_reg16(modrm.rm_field, src_val);
        machine.write_reg16(modrm.reg_field, dst_val);
    } else {
        // XCHG [addr], reg16
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let mem_val = machine.read_phys_u16(addr);
        let reg_val = machine.read_reg16(modrm.reg_field);
        machine.write_phys_u16(addr, reg_val);
        machine.write_reg16(modrm.reg_field, mem_val);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn add_al_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm8 = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None); // продвигаем на 1 байт (imm8)
    let mut bytes = prev.to_vec();
    bytes.push(imm8);
    let al = machine.registers.al();
    let result = (al as u16) + (imm8 as u16);
    let result8 = result as u8;
    let cf = result > 0xFF;
    let zf = result8 == 0;
    let sf = (result8 & 0x80) != 0;
    let af = ((al & 0x0F) + (imm8 & 0x0F)) > 0x0F;
    let pf = result8.count_ones() % 2 == 0;
    let al_sign = (al as i8) < 0;
    let imm_sign = (imm8 as i8) < 0;
    let result_sign = (result8 as i8) < 0;
    let of = (al_sign == imm_sign) && (result_sign != al_sign);
    let mut flags = 0u16;
    if cf { flags |= 1 << 0; }   // CF
    if pf { flags |= 1 << 2; }   // PF
    if af { flags |= 1 << 4; }   // AF
    if zf { flags |= 1 << 6; }   // ZF
    if sf { flags |= 1 << 7; }   // SF
    if of { flags |= 1 << 11; }  // OF
    machine.registers.set_flags(flags);
    machine.registers.set_al(result8);
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/alu.rs
pub fn add_ax_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm16 = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(2));
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());
    
    let ax = machine.registers.ax();
    let res = (ax as u32) + (imm16 as u32);
    let result = res as u16;
    
    // Установка флагов
    let cf = res > 0xFFFF;
    let zf = result == 0;
    let sf = (result & 0x8000) != 0;
    let af = ((ax & 0x0F) + (imm16 & 0x0F)) > 0x0F;
    let pf = result.count_ones() % 2 == 0;
    let of = (((ax ^ imm16) & 0x8000) == 0) && ((ax ^ result) & 0x8000) != 0;
    
    let mut flags = 0u16;
    if cf { flags |= 1 << 0; }
    if pf { flags |= 1 << 2; }
    if af { flags |= 1 << 4; }
    if zf { flags |= 1 << 6; }
    if sf { flags |= 1 << 7; }
    if of { flags |= 1 << 11; }
    machine.registers.set_flags(flags);
    
    machine.registers.set_ax(result);
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/alu.rs
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

// libs/dos_core/src/instructions/alu.rs
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
            (result, DosMachine::compute_flags_u16(result, cf, of, af))
        }
        1 => { // OR r/m16, imm8
            let result = dst_val | imm;
            (result, DosMachine::compute_logical_flags_u16(result))
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
            (result, DosMachine::compute_flags_u16(result, cf, of, af))
        }
        3 => { // SBB r/m16, imm8
            let borrow_in = (flags & 1) != 0;
            let imm_with_borrow = imm as u32 + if borrow_in { 1 } else { 0 };
            let cf = (dst_val as u32) < imm_with_borrow;
            let res = (dst_val as u32).wrapping_sub(imm_with_borrow);
            let result = res as u16;
            let af = (dst_val & 0x0F) < ((imm & 0x0F) + if borrow_in { 1 } else { 0 });
            let of = (((dst_val ^ imm) & 0x8000) != 0) && (((dst_val ^ result) & 0x8000) != 0);
            (result, DosMachine::compute_flags_u16(result, cf, of, af))
        }
        4 => { // AND r/m16, imm8
            let result = dst_val & imm;
            (result, DosMachine::compute_logical_flags_u16(result))
        }
        5 => { // SUB r/m16, imm8
            let res = (dst_val as u32).wrapping_sub(imm as u32);
            let result = res as u16;
            let cf = dst_val < imm;
            let af = (dst_val & 0x0F) < (imm & 0x0F);
            let of = (((dst_val ^ imm) & 0x8000) != 0) && (((dst_val ^ result) & 0x8000) != 0);
            (result, DosMachine::compute_flags_u16(result, cf, of, af))
        }
        6 => { // XOR r/m16, imm8
            let result = dst_val ^ imm;
            (result, DosMachine::compute_logical_flags_u16(result))
        }
        7 => { // CMP r/m16, imm8 — как SUB, но не сохраняем результат
            let res = (dst_val as u32).wrapping_sub(imm as u32);
            let result = res as u16;
            let cf = dst_val < imm;
            let af = (dst_val & 0x0F) < (imm & 0x0F);
            let of = (((dst_val ^ imm) & 0x8000) != 0) && (((dst_val ^ result) & 0x8000) != 0);
            let flags = DosMachine::compute_flags_u16(result, cf, of, af);
            (dst_val, flags) // Возвращаем исходное значение (не сохраняем результат)
        }
        _ => unreachable!(),
    }
}

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
            new_flags = DosMachine::compute_flags_u16(result, cf, of, af);
            result
        }
        1 => { // ROR
            let result = value.rotate_right(count as u32);
            let cf = (result >> 15) != 0;
            let lsb_before = (value & 1) != 0;
            let msb_after = (result >> 15) != 0;
            let of = if count == 1 { lsb_before != msb_after } else { false };
            new_flags = DosMachine::compute_flags_u16(result, cf, of, af);
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
            new_flags = DosMachine::compute_flags_u16(result, new_cf, of, af);
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
            new_flags = DosMachine::compute_flags_u16(result, new_cf, of, af);
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
            new_flags = DosMachine::compute_flags_u16(result, cf, of, af);
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
            new_flags = DosMachine::compute_flags_u16(result, cf, of, af);
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
            new_flags = DosMachine::compute_flags_u16(result, cf, of, af);
            result
        }
        _ => unreachable!(),
    };
    (result, new_flags)
}
