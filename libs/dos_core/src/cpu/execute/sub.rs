use crate::{
    dispatch_op32,
    instructions::{alu, alu32},
};

pub(crate) fn sub(opcode: u8, machine: &mut crate::DosMachine, full_bytes: &[u8]) {
    match opcode {
        0x2A => alu::sub_r8_rm8(machine, &full_bytes), // 8-битный вариант
        0x2B => dispatch_op32!(
            machine,
            alu32::sub_r32_rm32(machine, &full_bytes),
            alu::sub_r16_rm16(machine, &full_bytes)
        ),
        0x2C => alu::sub_al_imm8(machine, &full_bytes),
        0x29 => dispatch_op32!(
            machine,
            alu32::sub_rm32_r32(machine, &full_bytes),
            alu::sub_rm16_r16(machine, &full_bytes)
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
