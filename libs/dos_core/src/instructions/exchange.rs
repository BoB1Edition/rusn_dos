// Ver: 1
use crate::{DosMachine, modrm::ModRm};

pub(crate) fn xchg_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    if modrm.is_register_mode() {
        // XCHG reg16, reg16
        let src_val = machine.read_reg16(modrm.reg_field);
        let dst_val = machine.read_reg16(modrm.rm_field);
        machine.write_reg16(modrm.rm_field, src_val);
        machine.write_reg16(modrm.reg_field, dst_val);
    } else {
        // XCHG [addr], reg16
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        let mem_val = machine.read_phys_u16(addr);
        let reg_val = machine.read_reg16(modrm.reg_field);
        machine.write_phys_u16(addr, reg_val);
        machine.write_reg16(modrm.reg_field, mem_val);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn xchg_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    if modrm.is_register_mode() {
        let src_val = machine.read_reg32(modrm.reg_field);
        let dst_val = machine.read_reg32(modrm.rm_field);
        machine.write_reg32(modrm.rm_field, src_val);
        machine.write_reg32(modrm.reg_field, dst_val);
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        let mem_val = machine.read_phys_u32(addr);
        let reg_val = machine.read_reg32(modrm.reg_field);
        machine.write_phys_u32(addr, reg_val);
        machine.write_reg32(modrm.reg_field, mem_val);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}