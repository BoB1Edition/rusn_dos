// Ver: 1 File: crate/x86/src/instructions/exchange.rs
use crate::cpu::X86Cpu;
use crate::modrm::ModRm;
use bus::Machine;

pub fn xchg_rm8_r8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let reg_val = cpu.read_reg8(modrm.reg_field);
    if modrm.is_register_mode() {
        let rm_val = cpu.read_reg8(modrm.rm_field);
        cpu.write_reg8(modrm.reg_field, rm_val);
        cpu.write_reg8(modrm.rm_field, reg_val);
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        let mem_val = machine.read_mem_u8(addr);
        cpu.write_reg8(modrm.reg_field, mem_val);
        machine.write_mem_u8(addr, reg_val);
    }
}

pub fn xchg_rm_r(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    if cpu.prefixes.has_operand_size {
        let reg_val = cpu.read_reg32(modrm.reg_field);
        if modrm.is_register_mode() {
            let rm_val = cpu.read_reg32(modrm.rm_field);
            cpu.write_reg32(modrm.rm_field, reg_val);
            cpu.write_reg32(modrm.reg_field, rm_val);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            let mem_val = machine.read_mem_u32(addr);
            machine.write_mem_u32(addr, reg_val);
            cpu.write_reg32(modrm.reg_field, mem_val);
        }
    } else {
        let reg_val = cpu.read_reg16(modrm.reg_field);
        if modrm.is_register_mode() {
            let rm_val = cpu.read_reg16(modrm.rm_field);
            cpu.write_reg16(modrm.rm_field, reg_val);
            cpu.write_reg16(modrm.reg_field, rm_val);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            let mem_val = machine.read_mem_u16(addr);
            machine.write_mem_u16(addr, reg_val);
            cpu.write_reg16(modrm.reg_field, mem_val);
        }
    }
}

pub fn xchg_ax_reg(cpu: &mut X86Cpu, reg: u8) {
    if cpu.prefixes.has_operand_size {
        let eax = cpu.read_reg32(0);
        let other = cpu.read_reg32(reg);
        cpu.write_reg32(0, other);
        cpu.write_reg32(reg, eax);
    } else {
        let ax = cpu.read_reg16(0);
        let other = cpu.read_reg16(reg);
        cpu.write_reg16(0, other);
        cpu.write_reg16(reg, ax);
    }
}
