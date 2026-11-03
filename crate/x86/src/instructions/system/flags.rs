// Ver: 1 File: crate/x86/src/instructions/system/flags.rs

use crate::cpu::X86Cpu;
use crate::flags;
use bus::Machine;

pub fn clc(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let mut f = cpu.registers.flags();
    f &= !flags::CF;
    cpu.registers.set_flags(f);
}

pub fn stc(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let mut f = cpu.registers.flags();
    f |= flags::CF;
    cpu.registers.set_flags(f);
}

pub fn cmc(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let mut f = cpu.registers.flags();
    f ^= flags::CF;
    cpu.registers.set_flags(f);
}

pub fn cld(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let mut f = cpu.registers.flags();
    f &= !flags::DF;
    cpu.registers.set_flags(f);
}

pub fn std(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let mut f = cpu.registers.flags();
    f |= flags::DF;
    cpu.registers.set_flags(f);
}

pub fn cli(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let mut f = cpu.registers.flags();
    f &= !flags::IF;
    cpu.registers.set_flags(f);
}

pub fn sti(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let mut f = cpu.registers.flags();
    f |= flags::IF;
    cpu.registers.set_flags(f);
}

pub fn sahf(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    const MASK: u16 = 0x00D5;
    let ah = cpu.registers.ah() as u16;
    let mut flags = cpu.registers.flags();
    flags = (flags & !MASK) | (ah & MASK);
    cpu.registers.set_flags(flags);
}

pub fn lahf(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let ah = (cpu.registers.flags() & 0x00D5) as u8 | 0x02;
    cpu.registers.set_ah(ah);
}
