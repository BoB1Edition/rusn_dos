// Ver: 2 File: ./libs/dos_core/src/instructions/extended32.rs
use crate::{DosMachine, modrm::ModRm, xchg_eax_reg32};

pub(crate) fn movzx_r32_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field) as u32
    } else {
        let phys_addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&phys_addr.to_le_bytes());
        log::debug!(
            "MOVZX1: Reading from phys_addr {:#06X}, override_segment={:?}, DS={:#04X}, ES={:#04X}",
            phys_addr,
            machine.override_segment,
            machine.registers.ds(),
            machine.registers.es()
        );
        let val = machine.read_phys_u16(phys_addr);
        log::debug!("MOVZX1: Read value {:#04X}", val);
        machine.read_phys_u16(phys_addr) as u32
    };

    let dst_reg = modrm.reg_field;
    machine.write_reg32(dst_reg, src_val);

    machine.log_instruction(csip, &bytes).ok();
}
