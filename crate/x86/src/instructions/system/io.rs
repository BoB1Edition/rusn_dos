// Ver: 1 File: crate/x86/src/instructions/system/io.rs

use crate::cpu::X86Cpu;
use bus::Machine;

pub fn in_al_dx(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let port = cpu.registers.dx();
    let val = machine.in_port(port);
    cpu.registers.set_al(val);
}

pub fn out_dx_al(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let port = cpu.registers.dx();
    let val = cpu.registers.al();
    machine.out_port(port, val);
}

pub fn in_al_imm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let port = cpu.fetch_u8(machine) as u16;
    let val = machine.in_port(port);
    cpu.registers.set_al(val);
}

pub fn out_imm8_al(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let port = cpu.fetch_u8(machine) as u16;
    machine.out_port(port, cpu.registers.al());
}

pub fn in_ax_imm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let port = cpu.fetch_u8(machine) as u16;
    read_port_ax(cpu, machine, port);
}

pub fn out_imm8_ax(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let port = cpu.fetch_u8(machine) as u16;
    write_port_ax(cpu, machine, port);
}

pub fn in_ax_dx(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    read_port_ax(cpu, machine, cpu.registers.dx());
}

pub fn out_dx_ax(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    write_port_ax(cpu, machine, cpu.registers.dx());
}

fn read_port_ax(cpu: &mut X86Cpu, machine: &mut dyn Machine, port: u16) {
    if cpu.prefixes.has_operand_size {
        let v = machine.in_port32(port);
        cpu.registers.set_eax(v);
    } else {
        let v = machine.in_port16(port);
        cpu.registers.set_ax(v);
    }
}

fn write_port_ax(cpu: &mut X86Cpu, machine: &mut dyn Machine, port: u16) {
    if cpu.prefixes.has_operand_size {
        let v = cpu.registers.eax();
        machine.out_port32(port, v);
    } else {
        let v = cpu.registers.ax();
        machine.out_port16(port, v);
    }
}
