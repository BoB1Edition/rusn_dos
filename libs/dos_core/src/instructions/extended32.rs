use crate::{DosMachine, modrm::ModRm};

pub fn movzx_r32_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    

    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        let src_val = machine.read_reg16(modrm.rm_field) as u32;
        machine.write_reg32(modrm.reg_field, src_val);
    } else {
        if modrm.mod_field != 0b00 || modrm.rm_field != 0b110 {
            log::error!("Unsupported memory mode in MOVZX r32, r/m16");
            machine.halted = true;
            return;
        }

        let disp16 = machine.read_u16(machine.registers.cs(), machine.registers.ip());
        machine.registers.step(Some(2));
        bytes.extend_from_slice(&disp16.to_le_bytes());

        let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        let offset = disp16;

        let src_val = machine.read_u16(segment, offset) as u32;

        machine.write_reg32(modrm.reg_field, src_val);
    }
    machine.log_instruction(&bytes).ok();
}
