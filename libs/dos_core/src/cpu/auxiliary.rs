use crate::DosMachine;


pub(crate) fn execute_rep_simple(
    machine: &mut DosMachine,
    full_bytes: &[u8],
    step: fn(&mut DosMachine, &[u8]),
) {
    if machine.has_rep_prefix {
        while machine.registers.cx() != 0 {
            step(machine, full_bytes);
            machine.registers.set_cx(machine.registers.cx().wrapping_sub(1));
        }
    } else {
        step(machine, full_bytes);
    }
}