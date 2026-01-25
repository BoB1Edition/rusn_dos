use crate::DosMachine;

pub fn movzx_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let _ = machine.log_instruction(&bytes);

    let modrm = crate::modrm::ModRm::from_byte(modrm_byte);

    // Поддерживаем только регистровый режим пока
    if !modrm.is_register_mode() {
        log::error!("Memory operand in MOVZX r16, r/m16 not supported yet");
        machine.halted = true;
        return;
    }

    let src_reg = modrm.rm_field;
    let dst_reg = modrm.reg_field;

    let src_val = machine.read_reg16(src_reg);
    // Zero-extend 16→16 — просто копируем
    machine.write_reg16(dst_reg, src_val);
}