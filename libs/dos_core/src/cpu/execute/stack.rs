use crate::{dispatch_op32, instructions::stack};

pub(crate) fn stack(opcode: u8, machine: &mut crate::DosMachine, full_bytes: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - full_bytes.len() as u16,
    ];
    match opcode {
        0x06 => {
            stack::push_es(machine);
            machine.log_instruction(csip, &full_bytes).ok();
        }
        0x07 => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::pop_es(machine);
        }
        0x0E => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::push_cs(machine);
        }
        0x1E => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::push_ds(machine);
        }
        0x1F => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::pop_ds(machine);
        }
        0x50 => stack::push_ax(machine, &full_bytes),
        0x51 => stack::push_cx(machine, &full_bytes),
        0x52 => stack::push_dx(machine, &full_bytes),
        0x53 => stack::push_bx(machine, &full_bytes),
        0x54 => stack::push_sp(machine, &full_bytes),
        0x55 => stack::push_bp(machine, &full_bytes),
        0x56 => stack::push_si(machine, &full_bytes),
        0x57 => stack::push_di(machine, &full_bytes),
        0x58 => stack::pop_ax(machine, &full_bytes),
        0x59 => stack::pop_cx(machine, &full_bytes),
        0x5A => stack::pop_dx(machine, &full_bytes),
        0x5B => stack::pop_bx(machine, &full_bytes),
        0x5C => stack::pop_sp(machine, &full_bytes),
        0x5D => stack::pop_bp(machine, &full_bytes),
        0x5E => stack::pop_si(machine, &full_bytes),
        0x5F => stack::pop_di(machine, &full_bytes),
        0x60 => dispatch_op32!(machine, stack::pushad(machine), stack::pusha(machine)),
        0x61 => dispatch_op32!(machine, stack::popad(machine), stack::popa(machine)),
        0x68 => dispatch_op32!(
            machine,
            stack::push_imm32(machine, &full_bytes),
            stack::push_imm16(machine, &full_bytes)
        ),
        0x8F => stack::pop_rm16(machine, &full_bytes),
        0x9C => dispatch_op32!(
            machine,
            stack::pushfd(machine, &full_bytes),
            stack::pushf(machine, &full_bytes)
        ),
        0x9D => dispatch_op32!(
            machine,
            stack::popfd(machine, &full_bytes),
            stack::popf(machine, &full_bytes)
        ),
        _ => {
            log::error!(
                "opcode {:#04x} should not have been in the function stack",
                opcode
            );
            machine.halted = true
        }
    }
}
