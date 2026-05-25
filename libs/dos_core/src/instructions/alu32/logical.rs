// Ver: 1 File: ./libs/dos_core/src/instructions/alu32/logical.rs
use crate::{DosMachine, flags, modrm::ModRm};

pub(crate) fn xor_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = machine.read_reg32(modrm.reg_field);

    if modrm.is_register_mode() {
        let dst_val = machine.read_reg32(modrm.rm_field);
        let result = dst_val ^ src_val;
        machine.write_reg32(modrm.rm_field, result);
        machine
            .registers
            .set_flags(flags::compute_logical_flags_u32(
                machine.registers.flags(),
                result,
            ));
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        let dst_val = machine.read_phys_u32(addr);
        let result = dst_val ^ src_val;
        machine.write_phys_u32(addr, result);
        machine
            .registers
            .set_flags(flags::compute_logical_flags_u32(
                machine.registers.flags(),
                result,
            ));
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn or_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        let src = machine.read_reg32(modrm.reg_field);
        let dst = machine.read_reg32(modrm.rm_field);
        let result = dst | src;
        machine.write_reg32(modrm.rm_field, result);
        let result = dst | src;
        //machine.write_phys_u32(phys_addr, result);
        machine
            .registers
            .set_flags(flags::compute_logical_flags_u32(
                machine.registers.flags(),
                result,
            ));
    } else if modrm.mod_field == 0b00 && modrm.rm_field == 0b101 {
        if !machine.has_address_size_prefix {
            log::error!("Invalid memory mode for OR without address-size prefix");
            machine.halted = true;
            return;
        }
        let addr = machine.read_instr_u32(machine.registers.ip());
        machine.registers.step(Some(4));
        bytes.extend_from_slice(&addr.to_le_bytes());

        let offset = (addr & 0xFFFF) as u16;
        let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);

        let src_val = machine.read_reg32(modrm.reg_field);
        let dst_val = machine.read_phys_u32(phys_addr);
        let result = dst_val | src_val;
        machine.write_phys_u32(phys_addr, result);
        machine
            .registers
            .set_flags(flags::compute_logical_flags_u32(
                machine.registers.flags(),
                result,
            ));
    } else {
        log::error!("Unsupported memory mode in OR r/m32, r32");
        machine.halted = true;
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn or_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    // Источник: r/m32 (регистр или память)
    let src_val = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        machine.read_phys_u32(addr)
    };

    // Приёмник: регистр из reg_field
    let dst_reg = modrm.reg_field;
    let dst_val = machine.read_reg32(dst_reg);
    let result = dst_val | src_val;

    // Запись результата в регистр-приёмник
    machine.write_reg32(dst_reg, result);

    // Установка флагов (логическая операция: CF=0, OF=0)
    machine
        .registers
        .set_flags(flags::compute_logical_flags_u32(
            machine.registers.flags(),
            result,
        ));

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn xor_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let src_val = machine.read_reg32(modrm.reg_field);
    if modrm.is_register_mode() {
        let dst_val = machine.read_reg32(modrm.rm_field);
        let result = dst_val ^ src_val;
        machine.write_reg32(modrm.rm_field, result);
        machine
            .registers
            .set_flags(flags::compute_logical_flags_u32(
                machine.registers.flags(),
                result,
            ));
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        let dst_val = machine.read_phys_u32(addr);
        let result = dst_val ^ src_val;
        machine.write_phys_u32(addr, result);
        machine
            .registers
            .set_flags(flags::compute_logical_flags_u32(
                machine.registers.flags(),
                result,
            ));
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn and_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = machine.read_reg32(modrm.reg_field);
    if modrm.is_register_mode() {
        let dst_val = machine.read_reg32(modrm.rm_field);
        let result = dst_val & src_val;
        machine.write_reg32(modrm.rm_field, result);
        machine
            .registers
            .set_flags(flags::compute_logical_flags_u32(
                machine.registers.flags(),
                result,
            ));
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        let dst_val = machine.read_phys_u32(addr);
        let result = dst_val & src_val;
        machine.write_phys_u32(addr, result);
        machine
            .registers
            .set_flags(flags::compute_logical_flags_u32(
                machine.registers.flags(),
                result,
            ));
    }

    machine.log_instruction(csip, &bytes).ok();
}

/// AND r32, r/m32 (Опкод 0x23 с префиксом 0x66)
pub(crate) fn and_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        machine.read_phys_u32(addr)
    };

    let dst_reg = modrm.reg_field;
    let dst_val = machine.read_reg32(dst_reg);
    let result = dst_val & src_val;

    machine.write_reg32(dst_reg, result); // ← В регистр

    machine
        .registers
        .set_flags(flags::compute_logical_flags_u32(
            machine.registers.flags(),
            result,
        ));
    machine.log_instruction(csip, &bytes).ok();
}

pub fn test_eax_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let imm32 = machine.read_instr_u32(machine.registers.ip());
    machine.registers.step(Some(4));
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm32.to_le_bytes());

    let eax = machine.registers.eax();
    let result = eax & imm32;

    machine
        .registers
        .set_flags(flags::compute_logical_flags_u32(
            machine.registers.flags(),
            result,
        ));
    machine.log_instruction(csip, &bytes).ok();
}

pub fn or_eax_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let imm32 = machine.read_instr_u32(machine.registers.ip());
    machine.registers.step(Some(4));
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm32.to_le_bytes());

    let eax = machine.registers.eax();
    let result = eax | imm32;
    machine.registers.set_eax(result);
    machine.registers.set_flags(flags::compute_logical_flags_u32(machine.registers.flags(), result));
    machine.log_instruction(csip, &bytes).ok();
}