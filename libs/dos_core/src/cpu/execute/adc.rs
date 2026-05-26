use crate::{dispatch_op32, instructions::{alu, alu32}};

pub(crate) fn adc(opcode: u8, machine: &mut crate::DosMachine, full_bytes: &[u8]) {
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
        0x10 => alu::adc_rm8_r8(machine, &full_bytes),
        0x11 => dispatch_op32!(
            machine,
            alu32::adc_rm32_r32(machine, &full_bytes),
            alu::adc_rm16_r16(machine, &full_bytes)
        ),
        0x13 => dispatch_op32!(
            machine,
            alu32::adc_r32_rm32(machine, &full_bytes),
            alu::adc_r16_rm16(machine, &full_bytes)
        ),
        _ => {
            log::error!(
                "opcode {:#04x} should not have been in the function adc",
                opcode
            );
            machine.halted = true
        }
    }
}