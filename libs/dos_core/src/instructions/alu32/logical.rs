use crate::{DosMachine, flags, modrm::ModRm};

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
        machine
            .registers
            .set_flags(flags::compute_logical_flags_u32(result));
    } else {
        // XOR [mem], reg32
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let dst_val = machine.read_phys_u32(addr);
        let result = dst_val ^ src_val;
        machine.write_phys_u32(addr, result);
        machine
            .registers
            .set_flags(flags::compute_logical_flags_u32(result));
    }

    machine.log_instruction(csip, &bytes).ok();
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
    machine
        .registers
        .set_flags(flags::compute_logical_flags_u32(result));

    machine.log_instruction(csip, &bytes).ok();
}

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
        machine
            .registers
            .set_flags(flags::compute_logical_flags_u32(result));
    } else {
        // XOR [mem], reg32
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let dst_val = machine.read_phys_u32(addr);
        let result = dst_val ^ src_val;
        machine.write_phys_u32(addr, result);
        machine
            .registers
            .set_flags(flags::compute_logical_flags_u32(result));
    }

    machine.log_instruction(csip, &bytes).ok();
}

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
        machine
            .registers
            .set_flags(flags::compute_logical_flags_u32(result));
    } else {
        // AND [mem], reg32
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let dst_val = machine.read_phys_u32(addr);
        let result = dst_val & src_val;
        machine.write_phys_u32(addr, result);
        machine
            .registers
            .set_flags(flags::compute_logical_flags_u32(result));
    }

    machine.log_instruction(csip, &bytes).ok();
}
