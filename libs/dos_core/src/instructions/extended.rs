// Ver: 2
use crate::{DosMachine, modrm::ModRm};

pub(crate) fn movzx_r16_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = crate::modrm::ModRm::from_byte(modrm_byte);
    let dst_reg = modrm.reg_field;

    let src_val = if modrm.is_register_mode() {
        machine.read_reg8(modrm.rm_field) as u16
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.read_phys_u8(addr) as u16
    };

    machine.write_reg16(dst_reg, src_val);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_reg32_crn(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    // В Real Mode CR0-CR4 недоступны. Возвращаем 0 для совместимости.
    let cr_val: u32 = match modrm.reg_field {
        0 => 0x0000_0000, // CR0
        2 => 0x0000_0000, // CR2
        3 => 0x0000_0000, // CR3
        4 => 0x0000_0000, // CR4
        _ => {
            log::warn!("MOV r32, CR{} is undefined/reserved", modrm.reg_field);
            0
        }
    };

    // Записываем значение в целевой GPR (поле rm_field)
    match modrm.rm_field {
        0 => machine.registers.set_eax(cr_val),
        1 => machine.registers.set_ecx(cr_val),
        2 => machine.registers.set_edx(cr_val),
        3 => machine.registers.set_ebx(cr_val),
        4 => machine.registers.set_esp(cr_val),
        5 => machine.registers.set_ebp(cr_val),
        6 => machine.registers.set_esi(cr_val),
        7 => machine.registers.set_edi(cr_val),
        _ => log::error!("Invalid modrm.rm_field for MOV CRn"),
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_crn_reg32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    // Читаем значение из общего регистра (поле r/m в ModR/M)
    let gpr_val = match modrm.rm_field {
        0 => machine.registers.eax(),
        1 => machine.registers.ecx(),
        2 => machine.registers.edx(),
        3 => machine.registers.ebx(),
        4 => machine.registers.esp(),
        5 => machine.registers.ebp(),
        6 => machine.registers.esi(),
        7 => machine.registers.edi(),
        _ => 0,
    };

    // Логируем попытку записи (поле reg в ModR/M указывает номер CR)
    match modrm.reg_field {
        0 => log::warn!(
            "MOV CR0, reg32: value={:#010x} — Ignored (Real Mode stub, PE bit not set)",
            gpr_val
        ),
        2 => log::warn!(
            "MOV CR2, reg32: value={:#010x} — Ignored (Real Mode stub)",
            gpr_val
        ),
        3 => log::warn!(
            "MOV CR3, reg32: value={:#010x} — Ignored (Real Mode stub)",
            gpr_val
        ),
        4 => log::warn!(
            "MOV CR4, reg32: value={:#010x} — Ignored (Real Mode stub)",
            gpr_val
        ),
        _ => log::warn!("MOV CR{}, reg32 is reserved/undefined", modrm.reg_field),
    }
    machine.log_instruction(csip, &bytes).ok();
}
