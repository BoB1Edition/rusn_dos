use crate::cpu::X86Cpu;
use crate::modrm::ModRm;
use bus::Machine;

pub fn moffs_addr(cpu: &mut X86Cpu, machine: &mut dyn Machine) -> u32 {
    let seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());

    if cpu.prefixes.has_address_size {
        let offset = cpu.fetch_u32(machine);
        cpu.phys_addr32(seg, offset)
    } else {
        let offset = cpu.fetch_u16(machine);
        cpu.phys_addr(seg, offset)
    }
}

pub fn mov_rm8_r8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let src_val = cpu.read_reg8(modrm.reg_field);
    if modrm.is_register_mode() {
        cpu.write_reg8(modrm.rm_field, src_val);
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.write_mem_u8(phys_addr, src_val);
    }
}

pub fn mov_r8_rm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let src_val = if modrm.is_register_mode() {
        cpu.read_reg8(modrm.rm_field)
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.read_mem_u8(phys_addr)
    };
    cpu.write_reg8(modrm.reg_field, src_val);
}

pub fn mov_rm16_r16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let src_val = cpu.read_reg16(modrm.reg_field);
    if modrm.is_register_mode() {
        cpu.write_reg16(modrm.rm_field, src_val);
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.write_mem_u16(phys_addr, src_val);
    }
}

pub fn mov_r16_rm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let src_val = if modrm.is_register_mode() {
        cpu.read_reg16(modrm.rm_field)
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.read_mem_u16(phys_addr)
    };
    cpu.write_reg16(modrm.reg_field, src_val);
}

pub fn mov_rm32_r32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let src_val = cpu.read_reg32(modrm.reg_field);
    if modrm.is_register_mode() {
        cpu.write_reg32(modrm.rm_field, src_val);
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.write_mem_u32(phys_addr, src_val);
    }
}

pub fn mov_r32_rm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let src_val = if modrm.is_register_mode() {
        cpu.read_reg32(modrm.rm_field)
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.read_mem_u32(phys_addr)
    };
    cpu.write_reg32(modrm.reg_field, src_val);
}
