// Ver: 4
use crate::{DosMachine, flags, modrm::ModRm};

/// OR r8, r/m8
pub fn or_r8_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg8(modrm.rm_field)
    } else {
        let offset = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap() as u16;
        let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        machine.read_phys_u8(phys_addr)
    };

    let dst_reg = modrm.reg_field;
    let dst_val = machine.read_reg8(dst_reg);
    let result = dst_val | src_val;
    machine.write_reg8(dst_reg, result);
    machine.registers.set_flags(flags::compute_logical_flags_u8(machine.registers.flags(), result));
    machine.log_instruction(csip, &bytes).ok();
}

/// XOR r8, r/m8
pub fn xor_r8_rm(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg8(modrm.rm_field)
    } else {
        let offset = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap() as u16;
        let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        machine.read_phys_u8(phys_addr)
    };

    let dst_val = machine.read_reg8(modrm.reg_field);
    let result = dst_val ^ src_val;
    machine.write_reg8(modrm.reg_field, result);
    machine.registers.set_flags(flags::compute_logical_flags_u8(machine.registers.flags(), result));
    machine.log_instruction(csip, &bytes).ok();
}

/// OR r/m8, r8
pub fn or_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = machine.read_reg8(modrm.reg_field);

    if modrm.is_register_mode() {
        let dst_val = machine.read_reg8(modrm.rm_field);
        let result = dst_val | src_val;
        machine.write_reg8(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_logical_flags_u8(machine.registers.flags(), result));
    } else {
        let offset = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap() as u16;
        let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        
        let dst_val = machine.read_phys_u8(phys_addr);
        let result = dst_val | src_val;
        machine.write_phys_u8(phys_addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u8(machine.registers.flags(), result));
    }
    machine.log_instruction(csip, &bytes).ok();
}

/// OR r/m16, r16
pub fn or_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = machine.read_reg16(modrm.reg_field);

    if modrm.is_register_mode() {
        let dst_val = machine.read_reg16(modrm.rm_field);
        let result = dst_val | src_val;
        machine.write_reg16(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    } else {
        let offset = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap() as u16;
        let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        
        let dst_val = machine.read_phys_u16(phys_addr);
        let result = dst_val | src_val;
        machine.write_phys_u16(phys_addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    }
    machine.log_instruction(csip, &bytes).ok();
}

/// OR r16, r/m16
pub fn or_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let offset = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap() as u16;
        let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        machine.read_phys_u16(phys_addr)
    };

    let dst_reg = modrm.reg_field;
    let dst_val = machine.read_reg16(dst_reg);
    let result = dst_val | src_val;
    machine.write_reg16(dst_reg, result);
    machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    machine.log_instruction(csip, &bytes).ok();
}

/// AND r/m16, r16
pub fn and_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = machine.read_reg16(modrm.reg_field);

    if modrm.is_register_mode() {
        let dst_val = machine.read_reg16(modrm.rm_field);
        let result = dst_val & src_val;
        machine.write_reg16(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    } else {
        let offset = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap() as u16;
        let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        
        let dst_val = machine.read_phys_u16(phys_addr);
        let result = dst_val & src_val;
        machine.write_phys_u16(phys_addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    }
    machine.log_instruction(csip, &bytes).ok();
}

/// XOR r/m16, r16
pub fn xor_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = machine.read_reg16(modrm.reg_field);

    if modrm.is_register_mode() {
        let dst_val = machine.read_reg16(modrm.rm_field);
        let result = dst_val ^ src_val;
        machine.write_reg16(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    } else {
        let offset = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap() as u16;
        let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        
        let dst_val = machine.read_phys_u16(phys_addr);
        let result = dst_val ^ src_val;
        machine.write_phys_u16(phys_addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    }
    machine.log_instruction(csip, &bytes).ok();
}

/// XOR r16, r/m16
pub fn xor_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = machine.read_reg16(modrm.reg_field);

    if modrm.is_register_mode() {
        let dst_val = machine.read_reg16(modrm.rm_field);
        let result = dst_val ^ src_val;
        machine.write_reg16(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    } else {
        let offset = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap() as u16;
        let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        
        let dst_val = machine.read_phys_u16(phys_addr);
        let result = dst_val ^ src_val;
        machine.write_phys_u16(phys_addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    }
    machine.log_instruction(csip, &bytes).ok();
}

/// TEST r/m16, r16
pub fn test_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = machine.read_reg16(modrm.reg_field);
    let dst_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let offset = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap() as u16;
        let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        machine.read_phys_u16(phys_addr)
    };

    let result = dst_val & src_val;
    machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    machine.log_instruction(csip, &bytes).ok();
}

/// OR AL, imm8
pub fn or_al_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let imm8 = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(imm8);

    let al = machine.registers.al();
    let result = al | imm8;
    machine.registers.set_al(result);
    machine.registers.set_flags(flags::compute_logical_flags_u8(machine.registers.flags(), result));
    machine.log_instruction(csip, &bytes).ok();
}

/// XOR r/m8, r8
pub fn xor_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = machine.read_reg8(modrm.reg_field);

    if modrm.is_register_mode() {
        let dst_val = machine.read_reg8(modrm.rm_field);
        let result = dst_val ^ src_val;
        machine.write_reg8(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_logical_flags_u8(machine.registers.flags(), result));
    } else {
        let offset = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap() as u16;
        let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        
        let dst_val = machine.read_phys_u8(phys_addr);
        let result = dst_val ^ src_val;
        machine.write_phys_u8(phys_addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u8(machine.registers.flags(), result));
    }
    machine.log_instruction(csip, &bytes).ok();
}

/// AND AL, imm8
pub fn and_al_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let imm8 = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(imm8);

    let al = machine.registers.al();
    let result = al & imm8;
    machine.registers.set_al(result);
    machine.registers.set_flags(flags::compute_logical_flags_u8(machine.registers.flags(), result));
    machine.log_instruction(csip, &bytes).ok();
}

/// AND r/m8, r8
pub fn and_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = machine.read_reg8(modrm.reg_field);

    if modrm.is_register_mode() {
        let dst_val = machine.read_reg8(modrm.rm_field);
        let result = dst_val & src_val;
        machine.write_reg8(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_logical_flags_u8(machine.registers.flags(), result));
    } else {
        let offset = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap() as u16;
        let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        
        let dst_val = machine.read_phys_u8(phys_addr);
        let result = dst_val & src_val;
        machine.write_phys_u8(phys_addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u8(machine.registers.flags(), result));
    }
    machine.log_instruction(csip, &bytes).ok();
}