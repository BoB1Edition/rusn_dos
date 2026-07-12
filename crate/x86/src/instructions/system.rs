// Ver: 1 File: crate/x86/src/instructions/system.rs
use bus::Machine;
use crate::cpu::X86Cpu;
use crate::flags;

pub fn nop(_cpu: &mut X86Cpu, _machine: &mut dyn Machine) {}

pub fn hlt(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    log::info!("HLT executed. Halting CPU.");
    cpu.halted = true;
}

pub fn int(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let vector = cpu.fetch_u8(machine);
    
    // Читаем адрес обработчика из IVT (физическая память 0x00000)
    let ivt_addr = (vector as u32) * 4;
    let handler_ip = machine.read_mem_u16(ivt_addr);
    let handler_cs = machine.read_mem_u16(ivt_addr + 2);

    // PUSHF, PUSH CS, PUSH IP
    let ss_base = cpu.phys_addr(cpu.registers.ss(), 0);
    let mut sp = cpu.registers.sp();

    sp = sp.wrapping_sub(2);
    machine.write_mem_u16(ss_base.wrapping_add(sp as u32), cpu.registers.flags());
    
    sp = sp.wrapping_sub(2);
    machine.write_mem_u16(ss_base.wrapping_add(sp as u32), cpu.registers.cs());
    
    sp = sp.wrapping_sub(2);
    machine.write_mem_u16(ss_base.wrapping_add(sp as u32), cpu.registers.ip());

    cpu.registers.set_sp(sp);

    // Сбрасываем IF и TF
    let mut f = cpu.registers.flags();
    f &= !(flags::IF | flags::TF);
    cpu.registers.set_flags(f);

    // Переход
    cpu.registers.set_cs(handler_cs);
    cpu.registers.set_ip(handler_ip);
}

pub fn iret(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let ss_base = cpu.phys_addr(cpu.registers.ss(), 0);
    let mut sp = cpu.registers.sp();

    let ip = machine.read_mem_u16(ss_base.wrapping_add(sp as u32));
    sp = sp.wrapping_add(2);

    let cs = machine.read_mem_u16(ss_base.wrapping_add(sp as u32));
    sp = sp.wrapping_add(2);

    let f = machine.read_mem_u16(ss_base.wrapping_add(sp as u32));
    sp = sp.wrapping_add(2);

    cpu.registers.set_ip(ip);
    cpu.registers.set_cs(cs);
    cpu.registers.set_flags(f);
    cpu.registers.set_sp(sp);
}

pub fn in_al_dx(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let port = cpu.registers.dx();
    let val = machine.in_port(port);
    cpu.registers.set_al(val);
}

pub fn out_imm8_al(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let port = cpu.fetch_u8(machine) as u16;
    let val = cpu.registers.al();
    machine.out_port(port, val);
}

pub fn in_al_imm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let port = cpu.fetch_u8(machine) as u16;
    let val = machine.in_port(port);
    cpu.registers.set_al(val);
}

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