use crate::{dispatch_op32, instructions::{alu, alu32}};



pub(crate) fn cmp(opcode: u8, machine: &mut crate::DosMachine, full_bytes: &[u8]) {
    match opcode {
        0x38 => alu::cmp_rm8_r8(machine, &full_bytes),
        0x39 => dispatch_op32!(
            machine,
            alu32::cmp_rm32_r32(machine, &full_bytes),
            alu::cmp_rm16_r16(machine, &full_bytes)
        ),
        0x3A => alu::cmp_r8_rm8(machine, &full_bytes),
        0x3B => dispatch_op32!(
            machine,
            alu32::cmp_r32_rm32(machine, &full_bytes),
            alu::cmp_r16_rm16(machine, &full_bytes)
        ),
        0x3C => alu::cmp_al_imm8(machine, &full_bytes),
        0x3D => dispatch_op32!(
            machine,
            alu32::cmp_eax_imm32(machine, &full_bytes),
            alu::cmp_ax_imm16(machine, &full_bytes)
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

pub(crate) fn test(opcode: u8, machine: &mut crate::DosMachine, full_bytes: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - full_bytes.len() as u16,
    ];
    match opcode {
        _ => {
            log::error!(
                "opcode {:#04x} should not have been in the function stack",
                opcode
            );
            machine.halted = true
        }
    }
}