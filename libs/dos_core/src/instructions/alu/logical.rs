// Ver: 1 File: ./libs/dos_core/src/instructions/alu/logical.rs
use crate::{DosMachine, flags, modrm::ModRm};

/// OR r8, r/m8
pub(crate) fn or_r8_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg8(modrm.rm_field)
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        machine.read_phys_u8(addr)
    };

    let dst_reg = modrm.reg_field;
    let dst_val = machine.read_reg8(dst_reg);
    let result = dst_val | src_val;
    machine.write_reg8(dst_reg, result);
    machine.registers.set_flags(flags::compute_logical_flags_u8(machine.registers.flags(), result));
    machine.log_instruction(csip, &bytes).ok();
}

/// XOR r8, r/m8
pub(crate) fn xor_r8_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg8(modrm.rm_field)
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        //let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        //let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        machine.read_phys_u8(addr)
    };

    let dst_val = machine.read_reg8(modrm.reg_field);
    let result = dst_val ^ src_val;
    machine.write_reg8(modrm.reg_field, result);
    machine.registers.set_flags(flags::compute_logical_flags_u8(machine.registers.flags(), result));
    machine.log_instruction(csip, &bytes).ok();
}

/// OR r/m8, r8
pub(crate) fn or_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
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
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        let dst_val = machine.read_phys_u8(addr);
        let result = dst_val | src_val;
        machine.write_phys_u8(addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u8(machine.registers.flags(), result));
    }
    machine.log_instruction(csip, &bytes).ok();
}

/// OR r/m16, r16
pub(crate) fn or_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
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
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        let dst_val = machine.read_phys_u16(addr);
        let result = dst_val | src_val;
        machine.write_phys_u16(addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    }
    machine.log_instruction(csip, &bytes).ok();
}

/// OR r16, r/m16
pub(crate) fn or_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        machine.read_phys_u16(addr)
    };

    let dst_reg = modrm.reg_field;
    let dst_val = machine.read_reg16(dst_reg);
    let result = dst_val | src_val;
    machine.write_reg16(dst_reg, result);
    machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    machine.log_instruction(csip, &bytes).ok();
}

/// AND r/m16, r16
pub(crate) fn and_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
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
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        let dst_val = machine.read_phys_u16(addr);
        let result = dst_val & src_val;
        machine.write_phys_u16(addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    }
    machine.log_instruction(csip, &bytes).ok();
}

/// XOR r/m16, r16
pub(crate) fn xor_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
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
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        let dst_val = machine.read_phys_u16(addr);
        let result = dst_val ^ src_val;
        machine.write_phys_u16(addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    }
    machine.log_instruction(csip, &bytes).ok();
}

/// XOR r16, r/m16
pub(crate) fn xor_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
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
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        let dst_val = machine.read_phys_u16(addr);
        let result = dst_val ^ src_val;
        machine.write_phys_u16(addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    }
    machine.log_instruction(csip, &bytes).ok();
}

/// TEST r/m16, r16
pub(crate) fn test_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
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
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        machine.read_phys_u16(addr)
    };

    let result = dst_val & src_val;
    machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    machine.log_instruction(csip, &bytes).ok();
}

/// OR AL, imm8
pub(crate) fn or_al_imm8(machine: &mut DosMachine, prev: &[u8]) {
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
pub(crate) fn xor_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
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
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        let dst_val = machine.read_phys_u8(addr);
        let result = dst_val ^ src_val;
        machine.write_phys_u8(addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u8(machine.registers.flags(), result));
    }
    machine.log_instruction(csip, &bytes).ok();
}

/// AND AL, imm8
pub(crate) fn and_al_imm8(machine: &mut DosMachine, prev: &[u8]) {
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
pub(crate) fn and_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
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
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        let dst_val = machine.read_phys_u8(addr);
        let result = dst_val & src_val;
        machine.write_phys_u8(addr, result);
        machine.registers.set_flags(flags::compute_logical_flags_u8(machine.registers.flags(), result));
    }
    machine.log_instruction(csip, &bytes).ok();
}

/// AND r16, r/m16 (Опкод 0x23)
/// Результат записывается в РЕГИСТР (reg_field)
pub(crate) fn and_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let src_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        machine.read_phys_u16(addr)
    };
    let dst_reg = modrm.reg_field;
    let dst_val = machine.read_reg16(dst_reg);
    let result = dst_val & src_val;
    machine.write_reg16(dst_reg, result);
    
    machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn test_ax_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let imm16 = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2));
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());
    
    let ax = machine.registers.ax();
    let result = ax & imm16;
    machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    machine.log_instruction(csip, &bytes).ok();
}

/// CMP r/m8, r8 — Опкод 0x38
/// Операция: dst (r/m8) - src (r8). Результат не сохраняется, только флаги.
pub(crate) fn cmp_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = machine.read_reg8(modrm.reg_field);

    let dst_val = if modrm.is_register_mode() {
        machine.read_reg8(modrm.rm_field)
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        machine.read_phys_u8(addr)
    };

    let result = dst_val.wrapping_sub(src_val);
    let cf = dst_val < src_val;
    let af = (dst_val & 0x0F) < (src_val & 0x0F);
    let of = ((dst_val ^ src_val) & (dst_val ^ result)) & 0x80 != 0;

    machine.registers.set_flags(flags::compute_flags_u8(machine.registers.flags(), result, cf, of, af));
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn or_ax_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let imm16 = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2));
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());

    let ax = machine.registers.ax();
    let result = ax | imm16;
    machine.registers.set_ax(result);
    machine.registers.set_flags(flags::compute_logical_flags_u16(machine.registers.flags(), result));
    machine.log_instruction(csip, &bytes).ok();
}