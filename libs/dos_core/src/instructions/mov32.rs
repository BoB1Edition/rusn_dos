use crate::machine::DosMachine;

pub fn mov_address_eax(machine: &mut DosMachine, prev: &[u8]) {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(prev);
    let addr = if machine.has_address_size_prefix {
        let linear_ip = (machine.registers.cs() as u32) * 16 + (machine.registers.ip() as u32);
        let addr = machine.read_phys_u32(linear_ip);
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.registers.step(Some(4));
        machine.has_address_size_prefix = false;
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
