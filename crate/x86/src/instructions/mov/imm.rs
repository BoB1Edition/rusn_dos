use crate::cpu::X86Cpu;
use crate::modrm::ModRm;
use bus::Machine;

use super::rm::moffs_addr;

pub fn mov_rm16_sreg(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let sreg_value = match modrm.reg_field {
        0 => cpu.registers.es(),
        1 => cpu.registers.cs(),
        2 => cpu.registers.ss(),
        3 => cpu.registers.ds(),
        4 => cpu.registers.fs(),
        5 => cpu.registers.gs(),
        _ => unreachable!(),
    };
    if modrm.is_register_mode() {
        cpu.write_reg16(modrm.rm_field, sreg_value);
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.write_mem_u16(phys_addr, sreg_value);
    }
}

pub fn mov_sreg_rm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let src_val = if modrm.is_register_mode() {
        cpu.read_reg16(modrm.rm_field)
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.read_mem_u16(phys_addr)
    };
    match modrm.reg_field {
        0 => cpu.registers.set_es(src_val),
        1 => {
            log::error!("Attempt to write to CS register");
            cpu.halted = true;
        }
        2 => {
            cpu.registers.set_ss(src_val);
            cpu.inhibit_interrupts = true;
        }
        3 => cpu.registers.set_ds(src_val),
        4 => cpu.registers.set_fs(src_val),
        5 => cpu.registers.set_gs(src_val),
        _ => {
            log::error!("Invalid segment register field");
            cpu.halted = true;
        }
    }
}

pub fn mov_rm8_imm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    if modrm.reg_field != 0 {
        log::error!("MOV r/m8, imm8: reg field {:#x} is not 0", modrm.reg_field);
        cpu.halted = true;
        return;
    }
    if modrm.is_register_mode() {
        let imm8 = cpu.fetch_u8(machine);
        cpu.write_reg8(modrm.rm_field, imm8);
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        let imm8 = cpu.fetch_u8(machine);
        machine.write_mem_u8(phys_addr, imm8);
    }
}

pub fn mov_rm_imm(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    if modrm.reg_field != 0 {
        log::error!("MOV r/m, imm: reg field {:#x} is not 0", modrm.reg_field);
        cpu.halted = true;
        return;
    }
    if cpu.prefixes.has_operand_size {
        if modrm.is_register_mode() {
            let imm = cpu.fetch_u32(machine);
            cpu.write_reg32(modrm.rm_field, imm);
        } else {
            let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            let imm = cpu.fetch_u32(machine);
            machine.write_mem_u32(phys_addr, imm);
        }
    } else if modrm.is_register_mode() {
        let imm = cpu.fetch_u16(machine);
        cpu.write_reg16(modrm.rm_field, imm);
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        let imm = cpu.fetch_u16(machine);
        machine.write_mem_u16(phys_addr, imm);
    }
}

pub fn mov_ax_moffs(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let addr = moffs_addr(cpu, machine);
    if cpu.prefixes.has_operand_size {
        let val = machine.read_mem_u32(addr);
        cpu.registers.set_eax(val);
    } else {
        let val = machine.read_mem_u16(addr);
        cpu.registers.set_ax(val);
    }
}

pub fn mov_moffs_ax(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let addr = moffs_addr(cpu, machine);
    if cpu.prefixes.has_operand_size {
        let val = cpu.registers.eax();
        machine.write_mem_u32(addr, val);
    } else {
        let val = cpu.registers.ax();
        machine.write_mem_u16(addr, val);
    }
}

pub fn lea(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    if modrm.is_register_mode() {
        log::error!("LEA with register operand");
        cpu.halted = true;
        return;
    }
    let offset = modrm.resolve_offset(cpu, machine, cpu.prefixes.has_address_size);
    if cpu.prefixes.has_operand_size {
        cpu.write_reg32(modrm.reg_field, offset);
    } else {
        cpu.write_reg16(modrm.reg_field, offset as u16);
    }
}

pub fn les(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    load_far(cpu, machine, true);
}

pub fn lds(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    load_far(cpu, machine, false);
}

fn load_far(cpu: &mut X86Cpu, machine: &mut dyn Machine, es: bool) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    if modrm.is_register_mode() {
        log::error!("LDS/LES with register operand");
        cpu.halted = true;
        return;
    }
    let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
    if cpu.prefixes.has_operand_size {
        let off = machine.read_mem_u32(addr);
        let seg = machine.read_mem_u16(addr.wrapping_add(4));
        cpu.write_reg32(modrm.reg_field, off);
        if es {
            cpu.registers.set_es(seg);
        } else {
            cpu.registers.set_ds(seg);
        }
    } else {
        let off = machine.read_mem_u16(addr);
        let seg = machine.read_mem_u16(addr.wrapping_add(2));
        cpu.write_reg16(modrm.reg_field, off);
        if es {
            cpu.registers.set_es(seg);
        } else {
            cpu.registers.set_ds(seg);
        }
    }
}

pub fn mov_al_address(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let addr = moffs_addr(cpu, machine);
    let val = machine.read_mem_u8(addr);
    cpu.registers.set_al(val);
}

pub fn mov_ax_address(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let addr = moffs_addr(cpu, machine);
    if cpu.prefixes.has_operand_size {
        let val = machine.read_mem_u32(addr);
        cpu.registers.set_eax(val);
    } else {
        let val = machine.read_mem_u16(addr);
        cpu.registers.set_ax(val);
    }
}

pub fn mov_address_al(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let addr = moffs_addr(cpu, machine);
    let val = cpu.registers.al();
    machine.write_mem_u8(addr, val);
}

pub fn mov_address_ax(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let addr = moffs_addr(cpu, machine);
    if cpu.prefixes.has_operand_size {
        let val = cpu.registers.eax();
        machine.write_mem_u32(addr, val);
    } else {
        let val = cpu.registers.ax();
        machine.write_mem_u16(addr, val);
    }
}
