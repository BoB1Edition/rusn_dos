// Ver: 1 File: crate/x86/src/instructions/system/interrupt.rs

use crate::cpu::X86Cpu;
use crate::flags;
use bus::Machine;

/// INT imm8: вызов программного прерывания с младшими 16 битами вектора.
pub fn int(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let vector = cpu.fetch_u8(machine);
    software_interrupt(cpu, machine, vector);
}

/// Общая обработка программного прерывания: диспетчер машины либо IVT.
pub fn software_interrupt(cpu: &mut X86Cpu, machine: &mut dyn Machine, vector: u8) {
    if machine.handle_software_interrupt(vector) {
        cpu.pending_interrupt = Some(vector);
        return;
    }

    let ivt_addr = (vector as u32) * 4;
    let handler_ip = machine.read_mem_u16(ivt_addr);
    let handler_cs = machine.read_mem_u16(ivt_addr + 2);

    let flags = cpu.registers.flags();
    let cs = cpu.registers.cs();
    let ip = cpu.registers.ip();
    crate::instructions::stack::push16(cpu, machine, flags);
    crate::instructions::stack::push16(cpu, machine, cs);
    crate::instructions::stack::push16(cpu, machine, ip);

    let mut f = flags;
    f &= !(flags::IF | flags::TF);
    cpu.registers.set_flags(f);

    cpu.registers.set_cs(handler_cs);
    cpu.registers.set_ip(handler_ip);
}

pub fn int3(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    software_interrupt(cpu, machine, 3);
}

pub fn iret(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let ip = crate::instructions::stack::pop16(cpu, machine);
    let cs = crate::instructions::stack::pop16(cpu, machine);
    let f = crate::instructions::stack::pop16(cpu, machine);
    cpu.registers.set_ip(ip);
    cpu.registers.set_cs(cs);
    cpu.registers.set_flags(f);
}

pub fn into(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    if flags::test_of(cpu.registers.flags()) {
        software_interrupt(cpu, machine, 4);
    }
}

pub fn int1(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    log::error!(
        "INT1 (0xF1) not implemented at CS:IP = {:04X}:{:04X}",
        cpu.registers.cs(),
        cpu.registers.ip().wrapping_sub(1)
    );
    cpu.halted = true;
}
