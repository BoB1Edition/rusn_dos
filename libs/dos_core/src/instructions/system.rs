// Ver: 4
use log::warn;

use crate::{interrupts::bios, machine::DosMachine};

pub(crate) fn int(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let vector = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    bytes.push(vector);
    let _ = machine.log_instruction(csip, &bytes);
    machine.registers.step(None);
    match vector {
        0x21 => machine.handle_int21(),
        0x2F => machine.handle_int2f(),
        0x20 => machine.halted = true,
        0x10 => bios::handle_int10(machine),
        0x16 => bios::handle_int16(machine),
        _ => {
            println!("Unsupported interrupt: INT {:#02X}", vector);
            machine.halted = true;
        }
    }
}
