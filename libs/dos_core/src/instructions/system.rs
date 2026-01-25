use log::warn;

use crate::machine::DosMachine;

pub fn int(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let vector = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    bytes.push(vector);
    let _ = machine.log_instruction(csip, &bytes);
    machine.registers.step(None);
    match vector {
        0x21 => machine.handle_int21(),
        0x20 => machine.halted = true,
        _ => {
            warn!("Unsupported interrupt: INT {:#02X}", vector);
            machine.halted = true;
        }
    }
}
