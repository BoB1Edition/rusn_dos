// Ver: 1 File: crate/x86/src/instructions/system/regs.rs

use crate::cpu::X86Cpu;
use bus::Machine;

pub fn cbw(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    if cpu.prefixes.has_operand_size {
        let ax = cpu.registers.ax() as i16 as i32 as u32;
        cpu.registers.set_eax(ax);
    } else {
        let al = cpu.registers.al() as i8 as i16 as u16;
        cpu.registers.set_ax(al);
    }
}

pub fn cwd(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    if cpu.prefixes.has_operand_size {
        let eax = cpu.registers.eax() as i32;
        cpu.registers.set_edx(if eax < 0 { 0xFFFF_FFFF } else { 0 });
    } else {
        let ax = cpu.registers.ax() as i16;
        cpu.registers.set_dx(if ax < 0 { 0xFFFF } else { 0 });
    }
}

pub fn xlat(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
    if cpu.prefixes.has_address_size {
        // 32-битная адресация (префикс 0x67)
        let offset = cpu.registers.ebx().wrapping_add(cpu.registers.al() as u32);
        cpu.registers
            .set_al(machine.read_mem_u8(cpu.phys_addr32(seg, offset)));
    } else {
        // 16-битная адресация (по умолчанию)
        let offset = cpu.registers.bx().wrapping_add(cpu.registers.al() as u16);
        cpu.registers
            .set_al(machine.read_mem_u8(cpu.phys_addr(seg, offset)));
    }
}

pub fn pop_fs(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let v = crate::instructions::stack::pop16(cpu, machine);
    cpu.registers.set_fs(v);
}

pub fn push_fs(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let v = cpu.registers.fs();
    crate::instructions::stack::push16(cpu, machine, v);
}

pub fn push_gs(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let v = cpu.registers.gs();
    crate::instructions::stack::push16(cpu, machine, v);
}

pub fn pop_gs(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let v = crate::instructions::stack::pop16(cpu, machine);
    cpu.registers.set_gs(v);
}

pub fn movzx_b6(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = crate::modrm::ModRm::from_byte(cpu.fetch_u8(machine));
    let src = if modrm.is_register_mode() {
        cpu.read_reg8(modrm.rm_field) as u16
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.read_mem_u8(addr) as u16
    };
    if cpu.prefixes.has_operand_size {
        cpu.write_reg32(modrm.reg_field, src as u32);
    } else {
        cpu.write_reg16(modrm.reg_field, src);
    }
}

pub fn movzx_b7(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = crate::modrm::ModRm::from_byte(cpu.fetch_u8(machine));
    let src = if modrm.is_register_mode() {
        cpu.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.read_mem_u16(addr)
    };
    if cpu.prefixes.has_operand_size {
        cpu.write_reg32(modrm.reg_field, src as u32);
    } else {
        cpu.write_reg16(modrm.reg_field, src);
    }
}

pub fn movsx_b6(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = crate::modrm::ModRm::from_byte(cpu.fetch_u8(machine));
    let src = if modrm.is_register_mode() {
        cpu.read_reg8(modrm.rm_field) as i8 as i16 as u16
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.read_mem_u8(addr) as i8 as i16 as u16
    };
    if cpu.prefixes.has_operand_size {
        cpu.write_reg32(modrm.reg_field, src as i16 as i32 as u32);
    } else {
        cpu.write_reg16(modrm.reg_field, src);
    }
}

pub fn movsx_bf(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = crate::modrm::ModRm::from_byte(cpu.fetch_u8(machine));
    let src = if modrm.is_register_mode() {
        cpu.read_reg16(modrm.rm_field) as i16 as i32
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.read_mem_u16(addr) as i16 as i32
    };
    cpu.write_reg32(modrm.reg_field, src as u32);
}
