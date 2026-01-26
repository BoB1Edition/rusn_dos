use crate::{DosMachine, modrm::ModRm};

pub fn call_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);

    let target_addr = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u32(addr)
    };

    // В real mode: адрес усекается до 16 бит
    let target_ip = target_addr as u16;

    // PUSH текущего IP
    let current_ip = machine.registers.ip();
    machine.write_u16(
        machine.registers.ss(),
        machine.registers.sp().wrapping_sub(2),
        current_ip,
    );
    machine.registers.set_sp(machine.registers.sp().wrapping_sub(2));

    // JMP
    machine.registers.set_ip(target_ip);

    machine.log_instruction(csip, &bytes).ok();
}