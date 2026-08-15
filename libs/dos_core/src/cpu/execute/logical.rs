use crate::{
    dispatch_op32,
    instructions::{alu, alu32},
};

pub(crate) fn or(opcode: u8, machine: &mut crate::DosMachine, full_bytes: &[u8]) {
    match opcode {
        0x08 => alu::or_rm8_r8(machine, &full_bytes),
        0x09 => dispatch_op32!(
            machine,
            alu32::or_rm32_r32(machine, &full_bytes),
            alu::or_rm16_r16(machine, &full_bytes)
        ),
        0x0A => alu::or_r8_rm8(machine, &full_bytes),
        0x0B => dispatch_op32!(
            machine,
            alu32::or_r32_rm32(machine, &full_bytes),
            alu::or_r16_rm16(machine, &full_bytes)
        ),
        0x0C => alu::or_al_imm8(machine, &full_bytes),
        0x0D => dispatch_op32!(
            machine,
            alu32::or_eax_imm32(machine, &full_bytes),
            alu::or_ax_imm16(machine, &full_bytes)
        ),
        _ => {
            log::error!(
                "opcode {:#04x} should not have been in the function or",
                opcode
            );
            machine.halted = true
        }
    }
}

pub(crate) fn and(opcode: u8, machine: &mut crate::DosMachine, full_bytes: &[u8]) {
    match opcode {
        0x20 => alu::and_rm8_r8(machine, &full_bytes),
        0x21 => dispatch_op32!(
            machine,
            alu32::and_rm32_r32(machine, &full_bytes),
            alu::and_rm16_r16(machine, &full_bytes)
        ),
        0x22 => alu::and_r8_rm8(machine, &full_bytes),
        0x23 => dispatch_op32!(
            machine,
            alu32::and_r32_rm32(machine, &full_bytes),
            alu::and_r16_rm16(machine, &full_bytes)
        ),
        0x24 => alu::and_al_imm8(machine, &full_bytes),
        _ => {
            log::error!(
                "opcode {:#04x} should not have been in the function and",
                opcode
            );
            machine.halted = true
        }
    }
}

pub(crate) fn xor(opcode: u8, machine: &mut crate::DosMachine, full_bytes: &[u8]) {
    match opcode {
        0x30 => alu::xor_rm8_r8(machine, &full_bytes),
        0x31 => dispatch_op32!(
            machine,
            alu32::xor_rm32_r32(machine, &full_bytes),
            alu::xor_rm16_r16(machine, &full_bytes)
        ),
        0x32 => alu::xor_r8_rm8(machine, &full_bytes),
        0x33 => dispatch_op32!(
            machine,
            alu32::xor_r32_rm32(machine, &full_bytes),
            alu::xor_r16_rm16(machine, &full_bytes)
        ),
        _ => {
            log::error!(
                "opcode {:#04x} should not have been in the function and",
                opcode
            );
            machine.halted = true
        }
    }
}
