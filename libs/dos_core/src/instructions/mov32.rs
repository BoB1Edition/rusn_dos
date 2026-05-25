// Ver: 1 File: ./libs/dos_core/src/instructions/mov32.rs
use crate::{machine::DosMachine, modrm::ModRm};

pub(crate) fn mov_address_eax(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let addr32 = if machine.has_address_size_prefix {
        let addr32 = machine.read_instr_u32(machine.registers.ip());
        machine.registers.step(Some(4));
        addr32
    } else {
        let addr16 = machine.read_instr_u16(machine.registers.ip());
        machine.registers.step(Some(2));
        addr16 as u32
    };
    bytes.extend_from_slice(&addr32.to_le_bytes());
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let offset = (addr32 & 0xFFFF) as u16;
    machine.write_u32(segment, offset, machine.registers.eax());

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_eax_data(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(prev);
    let data = machine.read_instr_u32(machine.registers.ip());
    bytes.extend_from_slice(&data.to_le_bytes());
    machine.log_instruction(csip, &bytes).ok();
    machine.registers.step(Some(4));
    machine.registers.set_eax(data);
}

pub(crate) fn mov_edx_data(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(prev);
    let data = machine.read_instr_u32(machine.registers.ip());
    bytes.extend_from_slice(&data.to_le_bytes());
    machine.log_instruction(csip, &bytes).ok();
    machine.registers.step(Some(4));
    machine.registers.set_edx(data);
}

pub(crate) fn mov_eax_address16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let addr16 = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&addr16.to_le_bytes());

    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let value = machine.read_u32(segment, addr16);
    machine.registers.set_eax(value);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_ebx_data(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let data = machine.read_instr_u32(machine.registers.ip());
    bytes.extend_from_slice(&data.to_le_bytes());
    machine.log_instruction(csip, &bytes).ok();
    machine.registers.step(Some(4));
    machine.registers.set_ebx(data);
}

pub(crate) fn mov_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);
    let src_val = machine.read_reg32(modrm.reg_field);

    if modrm.is_register_mode() {
        machine.write_reg32(modrm.rm_field, src_val);
    } else {
        let phys_addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        //let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        //let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        machine.write_phys_u32(phys_addr, src_val);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
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
        machine.read_reg32(modrm.rm_field)
    } else {
        let phys_addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.read_phys_u32(phys_addr)
    };

    machine.write_reg32(modrm.reg_field, src_val);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_rm32_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.reg_field != 0 {
        log::error!("Invalid reg_field {} for opcode 0xC7", modrm.reg_field);
        machine.halted = true;
        return;
    }

    let imm32 = machine.read_instr_u32(machine.registers.ip());
    machine.registers.step(Some(4));
    bytes.extend_from_slice(&imm32.to_le_bytes());

    if modrm.is_register_mode() {
        machine.write_reg32(modrm.rm_field, imm32);
    } else {
        let phys_addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        //let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        //let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        machine.write_phys_u32(phys_addr, imm32);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_esi_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let imm32 = machine.read_instr_u32(machine.registers.ip());
    machine.registers.step(Some(4));
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm32.to_le_bytes());

    machine.registers.set_esi(imm32);

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_edi_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let imm32 = machine.read_instr_u32(machine.registers.ip());
    machine.registers.step(Some(4));
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm32.to_le_bytes());

    machine.registers.set_edi(imm32);

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_eax_address32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();

    let addr32 = if machine.has_address_size_prefix {
        let addr32 = machine.read_instr_u32(machine.registers.ip());
        machine.registers.step(Some(4));
        addr32
    } else {
        let addr16 = machine.read_instr_u16(machine.registers.ip());
        machine.registers.step(Some(2));
        addr16 as u32
    };
    bytes.extend_from_slice(&addr32.to_le_bytes());

    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let offset = (addr32 & 0xFFFF) as u16;

    let value = machine.read_u32(segment, offset);

    machine.registers.set_eax(value);

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn lea_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let sib_byte = if (modrm_byte & 0x07) == 4 && ((modrm_byte >> 6) & 0x03) != 3 {
        let sib = machine.read_instr_u8(machine.registers.ip());
        machine.registers.step(None);
        bytes.push(sib);
        Some(sib)
    } else {
        None
    };

    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        log::warn!(
            "LEA with register mode (mod=11) at {:#04x}:{:#04x} — emulating as MOV",
            machine.registers.cs(),
            machine.registers.ip()
        );
        let src_val = machine.read_reg32(modrm.rm_field);
        machine.write_reg32(modrm.reg_field, src_val);
    } else {
        let offset = compute_lea_offset_32(machine, &modrm, sib_byte, &mut bytes);
        machine.write_reg32(modrm.reg_field, offset);
    }

    machine.log_instruction(csip, &bytes).ok();
}

fn compute_lea_offset_32(
    machine: &mut DosMachine,
    modrm: &ModRm,
    sib_byte: Option<u8>,
    bytes: &mut Vec<u8>,
) -> u32 {
    let mod_field = (modrm.mod_field >> 6) & 0x03;
    let rm_field = modrm.rm_field & 0x07;

    // Базовый адрес
    let base = match rm_field {
        0 => machine.registers.eax(),
        1 => machine.registers.ecx(),
        2 => machine.registers.edx(),
        3 => machine.registers.ebx(),
        4 => {
            if let Some(sib) = sib_byte {
                let base_reg = sib & 0x07;
                match base_reg {
                    5 => {
                        if mod_field == 0 {
                            let disp32 = machine.read_instr_u32(machine.registers.ip());
                            machine.registers.step(Some(4));
                            bytes.extend_from_slice(&disp32.to_le_bytes());
                            return disp32;
                        } else {
                            machine.registers.ebp()
                        }
                    }
                    _ => match base_reg {
                        0 => machine.registers.eax(),
                        1 => machine.registers.ecx(),
                        2 => machine.registers.edx(),
                        3 => machine.registers.ebx(),
                        4 => 0, // нет базы
                        6 => machine.registers.esi(),
                        7 => machine.registers.edi(),
                        _ => unreachable!(),
                    },
                }
            } else {
                machine.registers.esp()
            }
        }
        5 => {
            if mod_field == 0 {
                let disp32 = machine.read_instr_u32(machine.registers.ip());
                machine.registers.step(Some(4));
                bytes.extend_from_slice(&disp32.to_le_bytes());
                return disp32;
            } else {
                machine.registers.ebp()
            }
        }
        6 => machine.registers.esi(),
        7 => machine.registers.edi(),
        _ => unreachable!(),
    };

    // Индекс и масштабирование (только при наличии SIB)
    let mut index_scaled = 0;
    if let Some(sib) = sib_byte {
        let index_reg = (sib >> 3) & 0x07;
        let scale = (sib >> 6) & 0x03;
        let index_value = match index_reg {
            0 => machine.registers.eax(),
            1 => machine.registers.ecx(),
            2 => machine.registers.edx(),
            3 => machine.registers.ebx(),
            4 => 0, // нет индекса
            5 => machine.registers.ebp(),
            6 => machine.registers.esi(),
            7 => machine.registers.edi(),
            _ => unreachable!(),
        };
        index_scaled = index_value << scale;
    }

    let displacement = match mod_field {
        0 => 0,
        1 => {
            let disp8 = machine.read_instr_u8(machine.registers.ip()) as i8 as i32;
            machine.registers.step(None);
            bytes.push(disp8 as u8);
            disp8 as u32
        }
        2 => {
            let disp32 = machine.read_instr_u32(machine.registers.ip());
            machine.registers.step(Some(4));
            bytes.extend_from_slice(&disp32.to_le_bytes());
            disp32
        }
        _ => 0,
    };

    base.wrapping_add(index_scaled).wrapping_add(displacement)
}

pub(crate) fn mov_ebp_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];

    let imm32 = machine.read_instr_u32(machine.registers.ip());
    machine.registers.step(Some(4));
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm32.to_le_bytes());
    machine.registers.set_ebp(imm32);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_esp_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let imm32 = machine.read_instr_u32(machine.registers.ip());
    machine.registers.step(Some(4));
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm32.to_le_bytes());
    machine.registers.set_esp(imm32);
    machine.log_instruction(csip, &bytes).ok();
}