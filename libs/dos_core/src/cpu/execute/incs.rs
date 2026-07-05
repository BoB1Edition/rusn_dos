use crate::instructions::incs;



pub(crate) fn incs(opcode: u8, machine: &mut crate::DosMachine, full_bytes: &[u8]) {
    match opcode {
        0x40 => incs::inc_ax(machine, &full_bytes),
        0x41 => incs::inc_cx(machine, &full_bytes),
        0x42 => incs::inc_dx(machine, &full_bytes),
        0x43 => incs::inc_bx(machine, &full_bytes),
        0x44 => incs::inc_sp(machine, &full_bytes),
        0x45 => incs::inc_bp(machine, &full_bytes),
        0x46 => incs::inc_si(machine, &full_bytes),
        0x47 => incs::inc_di(machine, &full_bytes),
        0x48 => incs::dec_ax(machine, &full_bytes),
        0x49 => incs::dec_cx(machine, &full_bytes),
        0x4A => incs::dec_dx(machine, &full_bytes),
        0x4B => incs::dec_bx(machine, &full_bytes),
        0x4C => incs::dec_sp(machine, &full_bytes),
        0x4D => incs::dec_bp(machine, &full_bytes),
        0x4E => incs::dec_si(machine, &full_bytes),
        0x4F => incs::dec_di(machine, &full_bytes),
        _ => {
            log::error!(
                "opcode {:#04x} should not have been in the function stack",
                opcode
            );
            machine.halted = true
        }
    }
}