// Ver: 1 File: ./libs/dos_core/src/instructions/extended32.rs
use crate::{DosMachine, modrm::ModRm, xchg_eax_reg32};

pub(crate) fn movzx_r32_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field) as u32
    } else {
        let offset = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap(); 
        let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        let phys_addr = ((segment as u32) << 4) + offset;
        machine.read_phys_u16(phys_addr) as u32
    };

    let dst_reg = modrm.reg_field;
    machine.write_reg32(dst_reg, src_val);

    machine.log_instruction(csip, &bytes).ok();
}