use crate::{DosMachine, flags, modrm::ModRm};


pub fn xor_r8_rm(machine: &mut DosMachine, prev: &[u8]) {
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
        .set_flags(flags::compute_logical_flags_u8(result));
    machine.log_instruction(csip, &bytes).ok();
}

pub fn or_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Источник: регистр из reg_field
    let src_val = machine.read_reg8(modrm.reg_field);
    
    // Приёмник: r/m8 (регистр или память)
    if modrm.is_register_mode() {
        // OR reg8, reg8
        let dst_val = machine.read_reg8(modrm.rm_field);
        let result = dst_val | src_val;
        machine.write_reg8(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_logical_flags_u8(result));
    } else {
        // OR [mem], reg8
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let dst_val = machine.read_phys_u8(addr);
        let result = dst_val | src_val;
        machine.write_phys_u8(addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u8(result));
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/alu.rs
pub fn or_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Источник: регистр из reg_field
    let src_val = machine.read_reg16(modrm.reg_field);
    
    // Приёмник: r/m16 (регистр или память)
    if modrm.is_register_mode() {
        // OR reg16, reg16
        let dst_val = machine.read_reg16(modrm.rm_field);
        let result = dst_val | src_val;
        machine.write_reg16(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(result));
    } else {
        // OR [mem], reg16
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let dst_val = machine.read_phys_u16(addr);
        let result = dst_val | src_val;
        machine.write_phys_u16(addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(result));
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn or_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Источник: r/m16 (регистр или память)
    let src_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u16(addr)
    };
    
    // Приёмник: регистр из reg_field
    let dst_reg = modrm.reg_field;
    let dst_val = machine.read_reg16(dst_reg);
    let result = dst_val | src_val;
    
    // Запись результата в регистр-приёмник
    machine.write_reg16(dst_reg, result);
    
    // Установка флагов (логическая операция: CF=0, OF=0)
    machine.registers.set_flags(flags::compute_logical_flags_u16(result));
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn and_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Источник: регистр из reg_field
    let src_val = machine.read_reg16(modrm.reg_field);
    
    // Приёмник: r/m16 (регистр или память)
    if modrm.is_register_mode() {
        // AND reg16, reg16
        let dst_val = machine.read_reg16(modrm.rm_field);
        let result = dst_val & src_val;
        machine.write_reg16(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(result));
    } else {
        // AND [mem], reg16
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let dst_val = machine.read_phys_u16(addr);
        let result = dst_val & src_val;
        machine.write_phys_u16(addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(result));
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn xor_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Источник: регистр из reg_field
    let src_val = machine.read_reg16(modrm.reg_field);
    
    // Приёмник: r/m16 (регистр или память)
    if modrm.is_register_mode() {
        // XOR reg16, reg16
        let dst_val = machine.read_reg16(modrm.rm_field);
        let result = dst_val ^ src_val;
        machine.write_reg16(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(result));
    } else {
        // XOR [mem], reg16
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let dst_val = machine.read_phys_u16(addr);
        let result = dst_val ^ src_val;
        machine.write_phys_u16(addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(result));
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn xor_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Источник: регистр из reg_field
    let src_val = machine.read_reg16(modrm.reg_field);
    
    // Приёмник: r/m16 (регистр или память)
    if modrm.is_register_mode() {
        // XOR reg16, reg16
        let dst_val = machine.read_reg16(modrm.rm_field);
        let result = dst_val ^ src_val;
        machine.write_reg16(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(result));
    } else {
        // XOR [mem], reg16
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let dst_val = machine.read_phys_u16(addr);
        let result = dst_val ^ src_val;
        machine.write_phys_u16(addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(result));
    }
    
    machine.log_instruction(csip, &bytes).ok();
}