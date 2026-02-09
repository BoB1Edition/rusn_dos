// Ver: 5
use crate::{machine::DosMachine, modrm::ModRm};

pub fn mov_address_eax(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(prev);
    let addr = if machine.has_address_size_prefix {
        let linear_ip = (machine.registers.cs() as u32) * 16 + (machine.registers.ip() as u32);
        let addr = machine.read_phys_u32(linear_ip);
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.registers.step(Some(4));
        addr
    } else {
        let linear_ip = (machine.registers.cs() as u32) * 16 + (machine.registers.ip() as u32);
        let addr = machine.read_phys_u16(linear_ip);
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.registers.step(Some(2));
        addr as u32
    };
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let offset = (addr & 0xFFFF) as u16;
    machine.write_u32(segment, offset, machine.registers.eax());
    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_eax_data(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(prev);
    let data = machine.read_u32(machine.registers.cs(), machine.registers.ip());
    bytes.extend_from_slice(&data.to_le_bytes());
    machine.log_instruction(csip, &bytes).ok();
    machine.registers.step(Some(4));
    machine.registers.set_eax(data);
}

pub fn mov_edx_data(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(prev);
    let data = machine.read_u32(machine.registers.cs(), machine.registers.ip());
    bytes.extend_from_slice(&data.to_le_bytes());
    machine.log_instruction(csip, &bytes).ok();
    machine.registers.step(Some(4));
    machine.registers.set_edx(data);
}

pub fn mov_ebx_data(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let data = machine.read_u32(machine.registers.cs(), machine.registers.ip());
    bytes.extend_from_slice(&data.to_le_bytes());
    machine.log_instruction(csip, &bytes).ok();
    machine.registers.step(Some(4));
    machine.registers.set_ebx(data);
}

pub fn mov_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);
    let src_val = machine.read_reg32(modrm.reg_field); // источник — регистр

    if modrm.is_register_mode() {
        // MOV reg32, reg32
        machine.write_reg32(modrm.rm_field, src_val);
    } else {
        // MOV [addr], reg32
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap(); // resolve_address уже обрабатывает seg override и BP→SS
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.write_phys_u32(addr, src_val);
    }

    machine.log_instruction(csip, &bytes).ok();
}

// mov32.rs
pub fn mov_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u32(addr)
    };

    machine.write_reg32(modrm.reg_field, src_val);
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov32.rs
pub fn mov_rm32_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Проверка подоперации: только /0 допустимо для опкода 0xC7
    if modrm.reg_field != 0 {
        log::error!("Invalid reg_field {} for opcode 0xC7", modrm.reg_field);
        machine.halted = true;
        return;
    }
    
    // Чтение непосредственного значения
    let imm32 = machine.read_u32(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(4));
    bytes.extend_from_slice(&imm32.to_le_bytes());
    
    if modrm.is_register_mode() {
        // MOV reg32, imm32
        machine.write_reg32(modrm.rm_field, imm32);
    } else {
        // MOV [addr], imm32
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.write_phys_u32(addr, imm32);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}