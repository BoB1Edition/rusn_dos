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