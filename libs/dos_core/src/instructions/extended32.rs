// Ver: 1
use crate::{DosMachine, modrm::ModRm};

pub fn movzx_r32_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field) as u32
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap(); 
        machine.read_phys_u16(addr) as u32
    };

    let dst_reg = modrm.reg_field;
    machine.write_reg32(dst_reg, src_val);

    machine.log_instruction(csip, &bytes).ok();
}