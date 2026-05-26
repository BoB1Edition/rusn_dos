use crate::{
    dispatch_op32,
    instructions::{alu, alu32},
};

pub(crate) fn sbb(opcode: u8, machine: &mut crate::DosMachine, full_bytes: &[u8]) {
    match opcode {
        0x18 => alu::sbb_rm8_r8(machine, &full_bytes),
        0x19 => dispatch_op32!(
            machine,
            alu32::sbb_rm32_r32(machine, &full_bytes),
            alu::sbb_rm16_r16(machine, &full_bytes)
        ),
        0x1B => dispatch_op32!(
            machine,
            alu32::sbb_r32_rm32(machine, &full_bytes),
            alu::sbb_r16_rm16(machine, &full_bytes)
        ),
        _ => {
            log::error!(
                "opcode {:#04x} should not have been in the function sbb",
                opcode
            );
            machine.halted = true
        }
    }
}
