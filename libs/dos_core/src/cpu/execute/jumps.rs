use crate::{dispatch_op32, instructions::{control, stack}};

pub(crate) fn jumps(opcode: u8, machine: &mut crate::DosMachine, full_bytes: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - full_bytes.len() as u16,
    ];
    match opcode {
        0x70 => control::jo_rel8(machine, &full_bytes),
        0x71 => control::jno_rel8(machine, &full_bytes),
        0x72 => control::jb(machine, &full_bytes),
        0x73 => control::jae_rel8(machine, &full_bytes),
        0x74 => control::jz(machine, &full_bytes),
        0x75 => control::jne_rel8(machine, &full_bytes),
        0x76 => control::jbe_rel8(machine, &full_bytes),
        0x77 => control::ja(machine, &full_bytes),
        0x78 => control::js_rel8(machine, &full_bytes),
        0x79 => control::jns_rel8(machine, &full_bytes),
        0x7C => control::jl_rel8(machine, &full_bytes),
        0x7D => control::jge_rel8(machine, &full_bytes),
        0x7E => control::jle_rel8(machine, &full_bytes),
        0x7F => control::jg_rel8(machine, &full_bytes),
        _ => {
            log::error!(
                "opcode {:#04x} should not have been in the function jumps",
                opcode
            );
            machine.halted = true
        }
    }
}

pub(crate) fn calls(opcode: u8, machine: &mut crate::DosMachine, full_bytes: &[u8]) {
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
