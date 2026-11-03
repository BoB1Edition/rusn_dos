// Ver: 1 File: crate/x86/src/instructions/system/control.rs

use crate::cpu::X86Cpu;
use bus::Machine;

pub fn group_0f01(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = crate::modrm::ModRm::from_byte(cpu.fetch_u8(machine));
    if modrm.reg_field <= 3 && modrm.is_register_mode() {
        log::error!("0F 01 /{} with mod=11 is undefined", modrm.reg_field);
        cpu.halted = true;
        return;
    }
    match modrm.reg_field {
        0 => {
            // SGDT
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u16(addr, cpu.gdtr_limit);
            machine.write_mem_u32(addr.wrapping_add(2), cpu.gdtr_base);
        }
        1 => {
            // SIDT
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u16(addr, cpu.idtr_limit);
            machine.write_mem_u32(addr.wrapping_add(2), cpu.idtr_base);
        }
        2 => {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            cpu.gdtr_limit = machine.read_mem_u16(addr);
            cpu.gdtr_base = machine.read_mem_u32(addr.wrapping_add(2)) & 0x00FF_FFFF;
        }
        3 => {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            cpu.idtr_limit = machine.read_mem_u16(addr);
            cpu.idtr_base = machine.read_mem_u32(addr.wrapping_add(2)) & 0x00FF_FFFF;
        }
        4 => {
            let msw = (cpu.cr0 & 0xFFFF) as u16;
            if modrm.is_register_mode() {
                cpu.write_reg16(modrm.rm_field, msw);
            } else {
                let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
                machine.write_mem_u16(addr, msw);
            }
        }
        6 => {
            let value = if modrm.is_register_mode() {
                cpu.read_reg16(modrm.rm_field)
            } else {
                let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
                machine.read_mem_u16(addr)
            };
            cpu.cr0 = (cpu.cr0 & 0xFFFF_0000) | value as u32;
        }
        _ => {
            log::warn!("Unhandled 0F 01 /{}", modrm.reg_field);
            if !modrm.is_register_mode() {
                let _ = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            }
        }
    }
}

pub fn mov_r32_cr(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = crate::modrm::ModRm::from_byte(cpu.fetch_u8(machine));
    let value = match modrm.reg_field {
        0 => cpu.cr0,
        _ => {
            log::error!("MOV r32, CR{}", modrm.reg_field);
            cpu.halted = true;
            return;
        }
    };
    cpu.write_reg32(modrm.rm_field, value);
}

pub fn mov_cr_r32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = crate::modrm::ModRm::from_byte(cpu.fetch_u8(machine));
    let value = cpu.read_reg32(modrm.rm_field);
    match modrm.reg_field {
        0 => cpu.cr0 = value,
        _ => {
            log::error!("MOV CR{}, r32", modrm.reg_field);
            cpu.halted = true;
        }
    }
}

pub fn setcc(cpu: &mut X86Cpu, machine: &mut dyn Machine, opcode: u8) {
    let modrm = crate::modrm::ModRm::from_byte(cpu.fetch_u8(machine));
    let cond = super::super::control::cond(cpu, opcode);
    let val = if cond { 1u8 } else { 0 };
    if modrm.is_register_mode() {
        cpu.write_reg8(modrm.rm_field, val);
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.write_mem_u8(addr, val);
    }
}
