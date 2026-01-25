use crate::machine::DosMachine;

pub fn call(machine: &mut DosMachine, prev: &[u8]) {
    let mut bytes = prev.to_vec();
    let rel16 = machine.read_u16(machine.registers.cs(), machine.registers.ip()) as i16;
    bytes.extend_from_slice(&rel16.to_le_bytes());
    let _ = machine.log_instruction(&bytes);
    let return_ip = machine.registers.ip().wrapping_add(2);

    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(machine.registers.ss(), machine.registers.sp(), return_ip);
    let new_ip = (return_ip as i32 + rel16 as i32) as u16;
    machine.registers.set_ip(new_ip);
}

pub fn retn(machine: &mut DosMachine, prev: &[u8]) {
    let _ = machine.log_instruction(prev);
    let ip = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_ip(ip);
}

pub fn jz(machine: &mut DosMachine, prev: &[u8]) {
    let mut bytes = prev.to_vec();
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    bytes.push(rel8 as u8);
    let _ = machine.log_instruction(&bytes);
    machine.registers.step(None);

    if (machine.registers.flags() & (1 << 6)) != 0 {
        let new_ip = (machine.registers.ip() as u32 + rel8 as u32) as u16;
        machine.registers.set_ip(new_ip);
    }
}
