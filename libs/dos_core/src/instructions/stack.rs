// Ver: 3
use crate::machine::DosMachine;

pub fn push_cs(machine: &mut DosMachine) {
    machine.registers.set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(machine.registers.ss(), machine.registers.sp(), machine.registers.cs());
}

pub fn push_ax(machine: &mut DosMachine) {
    machine.registers.set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(machine.registers.ss(), machine.registers.sp(), machine.registers.ax());
}

pub fn push_bx(machine: &mut DosMachine) {
    machine.registers.set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(machine.registers.ss(), machine.registers.sp(), machine.registers.bx());
}

pub fn pushf(machine: &mut DosMachine) {
    machine.registers.set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(machine.registers.ss(), machine.registers.sp(), machine.registers.flags());
}

pub fn pop_ds(machine: &mut DosMachine) { 
    let ds = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine.registers.set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_ds(ds);
}

pub fn pop_ax(machine: &mut DosMachine) {
    let ax = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine.registers.set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_ax(ax);
}

pub fn popf(machine: &mut DosMachine) {
    let flags = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine.registers.set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_flags(flags);
}