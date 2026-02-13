// Ver: 4
use crate::{DosMachine, instructions::control, modrm::ModRm};

pub fn call_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);

    let target_addr = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u32(addr)
    };

    // В real mode: адрес усекается до 16 бит
    let target_ip = target_addr as u16;

    // PUSH текущего IP
    let current_ip = machine.registers.ip();
    machine.write_u16(
        machine.registers.ss(),
        machine.registers.sp().wrapping_sub(2),
        current_ip,
    );
    machine.registers.set_sp(machine.registers.sp().wrapping_sub(2));

    // JMP
    machine.registers.set_ip(target_ip);

    machine.log_instruction(csip, &bytes).ok();
}

pub fn jmp_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);
    let target_addr = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.read_phys_u32(addr)
    };
    let target_ip = target_addr as u16;
    machine.registers.set_ip(target_ip);

    machine.log_instruction(csip, &bytes).ok();
}

pub fn call32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let rel32 = machine.read_u32(machine.registers.cs(), machine.registers.ip()) as i32;
    bytes.extend_from_slice(&rel32.to_le_bytes());
    machine.registers.step(Some(4));
    let offset16 = (rel32 & 0xFFFF) as i16;
    let return_ip = machine.registers.ip();
    machine.registers.set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(machine.registers.ss(), machine.registers.sp(), return_ip);
    let new_ip = (return_ip as i32 + offset16 as i32) as u16;
    machine.registers.set_ip(new_ip);
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn retn32(machine: &mut DosMachine, prev: &[u8]) {
    control::retn(machine, prev);
}

/// JZ/JE rel32 — условный переход при ZF=1 с 32-битным смещением
/// В реальном режиме результат усекается до 16 бит (IP — 16-битный регистр)
pub fn jz_rel32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    
    // Читаем 32-битное смещение (sign-extended)
    let rel32 = machine.read_u32(machine.registers.cs(), machine.registers.ip()) as i32;
    machine.registers.step(Some(4));
    bytes.extend_from_slice(&rel32.to_le_bytes());
    
    // Проверяем флаг ZF (бит 6)
    let zf = (machine.registers.flags() & (1 << 6)) != 0;
    
    if zf {
        // Вычисляем новый IP: текущий IP (после чтения смещения) + sign_extend(rel32)
        // В реальном режиме усекаем до 16 бит
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel32) as u16;
        machine.registers.set_ip(new_ip);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}