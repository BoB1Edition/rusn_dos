use crate::cpu::X86Cpu;
use crate::modrm::ModRm;
use bus::Machine;

pub fn read_rm8(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm) -> (u8, Option<u32>) {
    if modrm.is_register_mode() {
        (cpu.read_reg8(modrm.rm_field), None)
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        (machine.read_mem_u8(addr), Some(addr))
    }
}

pub fn write_rm8(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm, addr: Option<u32>, val: u8) {
    if let Some(addr) = addr {
        machine.write_mem_u8(addr, val);
    } else {
        cpu.write_reg8(modrm.rm_field, val);
    }
}

pub fn read_rm16(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm) -> (u16, Option<u32>) {
    if modrm.is_register_mode() {
        (cpu.read_reg16(modrm.rm_field), None)
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        (machine.read_mem_u16(addr), Some(addr))
    }
}

pub fn write_rm16(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm, addr: Option<u32>, val: u16) {
    if let Some(addr) = addr {
        machine.write_mem_u16(addr, val);
    } else {
        cpu.write_reg16(modrm.rm_field, val);
    }
}

pub fn read_rm32(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm) -> (u32, Option<u32>) {
    if modrm.is_register_mode() {
        (cpu.read_reg32(modrm.rm_field), None)
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        (machine.read_mem_u32(addr), Some(addr))
    }
}

pub fn write_rm32(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm, addr: Option<u32>, val: u32) {
    if let Some(addr) = addr {
        machine.write_mem_u32(addr, val);
    } else {
        cpu.write_reg32(modrm.rm_field, val);
    }
}
