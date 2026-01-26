use crate::{machine::DosMachine, modrm::ModRm};

pub fn call(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let rel16 = machine.read_u16(machine.registers.cs(), machine.registers.ip()) as i16;
    bytes.extend_from_slice(&rel16.to_le_bytes());
    let _ = machine.log_instruction(csip, &bytes);
    let return_ip = machine.registers.ip().wrapping_add(2);

    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(machine.registers.ss(), machine.registers.sp(), return_ip);
    let new_ip = (return_ip as i32 + rel16 as i32) as u16;
    machine.registers.set_ip(new_ip);
}

pub fn retn(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let _ = machine.log_instruction(csip, prev);
    let ip = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_ip(ip);
}

pub fn jz(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    bytes.push(rel8 as u8);
    let _ = machine.log_instruction(csip, &bytes);
    machine.registers.step(None);

    if (machine.registers.flags() & (1 << 6)) != 0 {
        let new_ip = (machine.registers.ip() as u32 + rel8 as u32) as u16;
        machine.registers.set_ip(new_ip);
    }
}

pub fn ja(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);
    machine.log_instruction(csip, &bytes).ok();

    let flags = machine.registers.flags();
    let cf = (flags & 0x0001) != 0;
    let zf = (flags & 0x0040) != 0;

    if !cf && !zf {
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel8 as i32) as u16;
        machine.registers.set_ip(new_ip);
    }
}

// control.rs
pub fn call_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);
    if modrm.is_register_mode() {
        // CALL reg16 — не поддерживается в real mode
        log::error!("CALL reg16 not supported in real mode");
        machine.halted = true;
        return;
    }

    // Поддерживаем только [disp16] (mod=00, rm=110)
    if modrm.mod_field == 0 && modrm.rm_field == 6 {
        let disp16 = machine.read_u16(machine.registers.cs(), machine.registers.ip());
        machine.registers.step(Some(2));
        bytes.extend_from_slice(&disp16.to_le_bytes());

        let addr = (machine.registers.ds() as u32) * 16 + (disp16 as u32);
        let target_ip = machine.read_phys_u16(addr);

        // PUSH IP
        let current_ip = machine.registers.ip();
        machine.write_u16(
            machine.registers.ss(),
            machine.registers.sp().wrapping_sub(2),
            current_ip,
        );
        machine
            .registers
            .set_sp(machine.registers.sp().wrapping_sub(2));

        // JMP
        machine.registers.set_ip(target_ip);

        machine.log_instruction(csip, &bytes).ok();
    } else {
        log::error!("Unsupported memory mode in CALL r/m16");
        machine.halted = true;
    }
}
