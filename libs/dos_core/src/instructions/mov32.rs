use crate::{machine::DosMachine, modrm::ModRm};

pub fn mov_address_eax(machine: &mut DosMachine, prev: &[u8]) {
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
    machine.log_instruction(&bytes).ok();
    machine.write_phys_u32(addr, machine.registers.eax());
}

pub fn mov_eax_data(machine: &mut DosMachine, prev: &[u8]) {
    let mut bytes = Vec::new();
    if !machine.has_address_size_prefix {
        bytes.extend_from_slice(prev);
        let data = machine.read_u32(machine.registers.cs(), machine.registers.ip());
        bytes.extend_from_slice(&data.to_le_bytes());
        machine.log_instruction(&bytes).ok();
        machine.registers.step(Some(4));
        machine.registers.set_eax(data);
    } else {
        machine.print_error_exit(prev.last().unwrap().clone());
    }
}

pub fn mov_ebx_data(machine: &mut DosMachine, prev: &[u8]) {
    if !machine.has_address_size_prefix {
        let mut bytes = prev.to_vec();
        let data = machine.read_u32(machine.registers.cs(), machine.registers.ip());
        bytes.extend_from_slice(&data.to_le_bytes());
        machine.log_instruction(&bytes).ok();
        machine.registers.step(Some(4));
        machine.registers.set_ebx(data);
    } else {
        machine.print_error_exit(prev.last().unwrap().clone());
    }
}

pub fn mov_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    

    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        // MOV reg32, reg32
        let src = machine.read_reg32(modrm.reg_field);
        machine.write_reg32(modrm.rm_field, src);
    } else if modrm.mod_field == 0b00 && modrm.rm_field == 0b101 {
        // MOV [disp32], reg32
        let addr = machine.read_u32(machine.registers.cs(), machine.registers.ip());
        machine.registers.step(Some(4));
        bytes.extend_from_slice(&addr.to_le_bytes());
        let src_val = machine.read_reg32(modrm.reg_field);
        machine.write_phys_u32(addr as u32, src_val);
    } else {
        machine.log_instruction(&bytes).ok();
        log::error!("Unsupported memory mode in MOV r/m32, r32");
        machine.halted = true;
    }
    machine.log_instruction(&bytes).ok();
}
