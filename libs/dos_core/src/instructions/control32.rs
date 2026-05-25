// Ver: 1 File: ./libs/dos_core/src/instructions/control32.rs
use crate::{DosMachine, flags, instructions::control, modrm::ModRm};

pub(crate) fn call_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
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
    let current_ip = machine.registers.ip();
    machine.write_u16(
        machine.registers.ss(),
        machine.registers.sp().wrapping_sub(2),
        current_ip,
    );
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_sub(2));
    machine.registers.set_ip(target_ip);

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jmp_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
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

pub(crate) fn call32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let rel32 = machine.read_instr_u32(machine.registers.ip()) as i32;
    bytes.extend_from_slice(&rel32.to_le_bytes());
    machine.registers.step(Some(4));
    let return_ip = machine.registers.ip();
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(machine.registers.ss(), machine.registers.sp(), return_ip);
    let new_ip = (return_ip as i32).wrapping_add(rel32) as u16;
    machine.registers.set_ip(new_ip);

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn retn32(machine: &mut DosMachine, prev: &[u8]) {
    control::retn(machine, prev);
}

/// JZ/JE rel32 — условный переход при ZF=1 с 32-битным смещением
/// В реальном режиме результат усекается до 16 бит (IP — 16-битный регистр)
pub(crate) fn jz_rel32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let rel32 = machine.read_instr_u32(machine.registers.ip()) as i32;
    machine.registers.step(Some(4));
    bytes.extend_from_slice(&rel32.to_le_bytes());
    let zf = (machine.registers.flags() & (flags::ZF)) != 0;

    if zf {
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel32) as u16;
        machine.registers.set_ip(new_ip);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jecxz_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    if machine.registers.ecx() == 0 {
        let new_eip = (machine.registers.ip() as i32).wrapping_add(rel8 as i32) as u32;
        machine.registers.set_ip((new_eip & 0xFFFF) as u16);
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn call_far_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let addr = if modrm.is_register_mode() {
        log::error!(
            "CALL far through register is undefined behavior at {:#04x}:{:#04x}",
            machine.registers.cs(),
            machine.registers.ip()
        );
        machine.halted = true;
        return;
    } else {
        modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap()
    };
    let ip_offset = machine.read_phys_u16(addr);
    let cs_segment = machine.read_phys_u16(addr.wrapping_add(2));
    let sp = machine.registers.sp().wrapping_sub(2);
    machine.registers.set_sp(sp);
    machine.write_u16(machine.registers.ss(), sp, machine.registers.cs());

    let sp = machine.registers.sp().wrapping_sub(2);
    machine.registers.set_sp(sp);
    machine.write_u16(machine.registers.ss(), sp, machine.registers.ip());
    machine.registers.set_cs(cs_segment);
    machine.registers.set_ip(ip_offset);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jmp_far_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let addr = if modrm.is_register_mode() {
        log::error!(
            "JMP far through register is undefined behavior at {:#04x}:{:#04x}",
            machine.registers.cs(),
            machine.registers.ip()
        );
        machine.halted = true;
        return;
    } else {
        modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap()
    };

    let ip_offset = machine.read_phys_u16(addr);
    let cs_segment = machine.read_phys_u16(addr.wrapping_add(2));

    machine.registers.set_cs(cs_segment);
    machine.registers.set_ip(ip_offset);

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jae_rel32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();

    let rel32 = machine.read_instr_u32(machine.registers.ip()) as i32;
    machine.registers.step(Some(4));
    bytes.extend_from_slice(&rel32.to_le_bytes());

    let cf = (machine.registers.flags() & flags::CF) == 0;

    if cf {
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel32) as u16;
        machine.registers.set_ip(new_ip);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jb_rel32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let mut bytes = prev.to_vec();
    
    let rel32 = machine.read_instr_u32(machine.registers.ip()) as i32;
    machine.registers.step(Some(4));
    bytes.extend_from_slice(&rel32.to_le_bytes());

    let cf = (machine.registers.flags() & flags::CF) != 0;
    if cf {
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel32) as u16;
        machine.registers.set_ip(new_ip);
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub fn bound_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    if modrm.is_register_mode() { machine.halted = true; return; }
    let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
    let low  = machine.read_phys_u32(addr) as i32;
    let high = machine.read_phys_u32(addr.wrapping_add(4)) as i32;
    let reg_val = machine.read_reg32(modrm.reg_field) as i32;

    if reg_val < low || reg_val > high {
        log::warn!("BOUND32 range exceeded");
        crate::instructions::system::int(machine, &[0xCD, 0x05]);
        return;
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn retn_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let imm16 = machine.read_instr_u16(machine.registers.ip()); // в 32-битном режиме imm16 тоже 2 байта
    bytes.extend_from_slice(&imm16.to_le_bytes());
    machine.registers.step(Some(2));

    let ip = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    let new_sp = machine.registers.sp().wrapping_add(2 + imm16);
    machine.registers.set_sp(new_sp);
    machine.registers.set_ip(ip);

    machine.log_instruction(csip, &bytes).ok();
}