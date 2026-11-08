// Ver: 1 File: crate/x86/src/instructions/system/misc.rs

use crate::cpu::X86Cpu;
use crate::flags;
use bus::Machine;

use super::interrupt::software_interrupt;

pub fn nop(_cpu: &mut X86Cpu, _machine: &mut dyn Machine) {}

pub fn hlt(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    log::info!("HLT executed. Halting CPU.");
    cpu.halted = true;
}

pub fn wait(_cpu: &mut X86Cpu, _machine: &mut dyn Machine) {}

pub fn arpl(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = crate::modrm::ModRm::from_byte(cpu.fetch_u8(machine));
    let src_rpl = cpu.read_reg16(modrm.reg_field) & 0x03;
    let (dst_old, addr) = if modrm.is_register_mode() {
        (cpu.read_reg16(modrm.rm_field), None)
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        (machine.read_mem_u16(addr), Some(addr))
    };
    let raised = src_rpl > (dst_old & 0x03);
    if raised {
        let dst_new = (dst_old & 0xFFFC) | src_rpl;
        if let Some(addr) = addr {
            machine.write_mem_u16(addr, dst_new);
        } else {
            cpu.write_reg16(modrm.rm_field, dst_new);
        }
    }
    let mut f = cpu.registers.flags();
    if raised {
        f |= flags::ZF;
    } else {
        f &= !flags::ZF;
    }
    cpu.registers.set_flags(f);
}

pub fn imul_imm(cpu: &mut X86Cpu, machine: &mut dyn Machine, imm8: bool) {
    let modrm = crate::modrm::ModRm::from_byte(cpu.fetch_u8(machine));
    if cpu.prefixes.has_operand_size {
        let src = if modrm.is_register_mode() {
            cpu.read_reg32(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u32(addr)
        };
        let imm = if imm8 {
            cpu.fetch_u8(machine) as i8 as i32 as u32
        } else {
            cpu.fetch_u32(machine)
        };
        let full = (src as i32 as i64) * (imm as i32 as i64);
        let result = full as u32;
        cpu.write_reg32(modrm.reg_field, result);
        set_mul_overflow(cpu, full != result as i32 as i64);
    } else {
        let src = if modrm.is_register_mode() {
            cpu.read_reg16(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u16(addr)
        };
        let imm = if imm8 {
            cpu.fetch_u8(machine) as i8 as i16 as u16
        } else {
            cpu.fetch_u16(machine)
        };
        let full = (src as i16 as i32) * (imm as i16 as i32);
        let result = full as u16;
        cpu.write_reg16(modrm.reg_field, result);
        set_mul_overflow(cpu, full != result as i16 as i32);
    }
}

fn set_mul_overflow(cpu: &mut X86Cpu, overflow: bool) {
    let mut f = cpu.registers.flags();
    if overflow {
        f |= flags::CF | flags::OF;
    } else {
        f &= !(flags::CF | flags::OF);
    }
    cpu.registers.set_flags(f);
}

pub fn aam(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let base = cpu.fetch_u8(machine);
    if base == 0 {
        software_interrupt(cpu, machine, 0);
        return;
    }
    let al = cpu.registers.al();
    cpu.registers.set_ah(al / base);
    let new_al = al % base;
    cpu.registers.set_al(new_al);
    let f = flags::compute_flags_u8(cpu.registers.flags(), new_al, false, false, false, false);
    let old_cf = cpu.registers.flags() & flags::CF;
    let old_of = cpu.registers.flags() & flags::OF;
    let old_af = cpu.registers.flags() & flags::AF;
    cpu.registers
        .set_flags((f & !(flags::CF | flags::OF | flags::AF)) | old_cf | old_of | old_af);
}

pub fn aad(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let base = cpu.fetch_u8(machine);
    let al = cpu
        .registers
        .ah()
        .wrapping_mul(base)
        .wrapping_add(cpu.registers.al());
    cpu.registers.set_ax(al as u16);
    let f = flags::compute_flags_u8(cpu.registers.flags(), al, false, false, false, false);
    let kept = cpu.registers.flags() & (flags::CF | flags::OF | flags::AF);
    cpu.registers
        .set_flags((f & !(flags::CF | flags::OF | flags::AF)) | kept);
}

pub fn salc(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    cpu.registers
        .set_al(if flags::test_cf(cpu.registers.flags()) {
            0xFF
        } else {
            0
        });
}

pub fn cpuid(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    match cpu.registers.eax() {
        0 => {
            cpu.registers.set_eax(1);
            cpu.registers.set_ebx(0x756e6547);
            cpu.registers.set_ecx(0x6c65746e);
            cpu.registers.set_edx(0x49656e69);
        }
        1 => {
            cpu.registers.set_eax(0x00000400);
            cpu.registers.set_ebx(0);
            cpu.registers.set_ecx(0);
            cpu.registers.set_edx(0x00000001);
        }
        _ => {
            cpu.registers.set_eax(0);
            cpu.registers.set_ebx(0);
            cpu.registers.set_ecx(0);
            cpu.registers.set_edx(0);
        }
    }
}

pub fn rdtsc(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    cpu.registers.set_eax(0);
    cpu.registers.set_edx(0);
}
