use crate::DosMachine;

pub fn movzx_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = crate::modrm::ModRm::from_byte(modrm_byte);

    let dst_reg = modrm.reg_field;

    let src_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u16(addr)
    };
    machine.write_reg16(dst_reg, src_val);
    machine.log_instruction(csip, &bytes).ok();
}
