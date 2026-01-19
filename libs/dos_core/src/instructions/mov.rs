use log::{error};

use crate::{machine::DosMachine, modrm::ModRm};

pub fn mov_ah(machine: &mut DosMachine, prev: &[u8]) {
    let mut bytes = prev.to_vec();
    let imm = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    bytes.push(imm);
    let _ = machine.log_instruction(&bytes);
    machine.registers.set_ah(imm);
    machine.registers.step(None);
}

pub fn mov_ax(machine: &mut DosMachine, prev: &[u8]) {
    let mut bytes = prev.to_vec();
    let imm = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    bytes.extend_from_slice(&imm.to_le_bytes());
    let _ = machine.log_instruction(&bytes);
    machine.registers.set_ax(imm);
    machine.registers.step(Some(2));
}

pub fn mov_dx(machine: &mut DosMachine, prev: &[u8]) {
    let mut bytes = prev.to_vec();
    let imm = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    bytes.extend_from_slice(&imm.to_le_bytes());
    let _ = machine.log_instruction(&bytes);
    machine.registers.set_dx(imm);
    machine.registers.step(Some(2));
}

pub fn mov(machine: &mut DosMachine, prev: &[u8]) {
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    machine.log_instruction(&bytes).ok();

    let modrm = ModRm::from_byte(modrm_byte);
    if !modrm.is_register_mode() {
        error!("Memory operand in MOV r/m16, Sreg not supported yet");
        machine.halted = true;
        return;
    }

    let sreg_value = machine.read_sreg(modrm.reg_field); // источник: сегментный регистр
    machine.write_reg16(modrm.rm_field, sreg_value);    // приёмник: общий регистр
}