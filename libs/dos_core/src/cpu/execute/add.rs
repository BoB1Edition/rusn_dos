use crate::{
    dispatch_op32,
    instructions::{alu, alu32},
};

pub(crate) fn add(opcode: u8, machine: &mut crate::DosMachine, full_bytes: &[u8]) {
    match opcode {
        0x00 => alu::add_rm8_r8(machine, &full_bytes),
        0x01 => dispatch_op32!(
            machine,
            alu32::add_rm32_r32(machine, &full_bytes),
            alu::add_rm16_r16(machine, &full_bytes)
        ),
        0x02 => alu::add_r8_rm8(machine, &full_bytes),
        0x03 => dispatch_op32!(
            machine,
            alu32::add_r32_rm32(machine, &full_bytes),
            alu::add_r16_rm16(machine, &full_bytes)
        ),
        0x04 => alu::add_al_imm8(machine, &full_bytes),
        0x05 => dispatch_op32!(
            machine,
            alu32::add_eax_imm32(machine, &full_bytes),
            alu::add_ax_imm16(machine, &full_bytes)
        ),
        _ => {
            log::error!(
                "opcode {:#04x} should not have been in the function add",
                opcode
            );
            machine.halted = true
        }
    }
}
