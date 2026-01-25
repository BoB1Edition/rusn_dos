use log::error;

use crate::{
    machine::{self, DosMachine},
    modrm::ModRm,
};

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

pub fn mov_rm16_sreg(machine: &mut DosMachine, prev: &[u8]) {
    let mut bytes = prev.to_vec();
    //if !machine.has_address_size_prefix {
        let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
        machine.registers.step(None);
        bytes.push(modrm_byte);
        machine.log_instruction(&bytes).ok();

        let modrm = ModRm::from_byte(modrm_byte);
        if !modrm.is_register_mode() {
            error!("Memory operand in MOV r/m16, Sreg not supported yet");
            machine.halted = true;
            return;
        }

        let sreg_value = machine.read_sreg(modrm.reg_field); // источник: сегментный регистр
        machine.write_reg16(modrm.rm_field, sreg_value); // приёмник: общий регистр
    /*} else {
        machine.print_error_exit(bytes.last().unwrap().clone());
    }*/
}

pub fn mov_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None); // прочитали ModR/M

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let _ = machine.log_instruction(&bytes);

    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        // MOV reg16, reg16
        let src_reg = modrm.reg_field;   // источник
        let dst_reg = modrm.rm_field;    // приёмник
        let src_val = machine.read_reg16(src_reg);
        machine.write_reg16(dst_reg, src_val);
    } else {
        // Память пока не поддерживается
        log::error!("Memory operand in MOV r/m16, r16 not supported yet");
        machine.halted = true;
    }
}