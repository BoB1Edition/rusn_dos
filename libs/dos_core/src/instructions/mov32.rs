// Ver: 1
use crate::{machine::DosMachine, modrm::ModRm};

pub fn mov_address_eax(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    
    // Читаем 32-битное смещение из [CS:IP]
    let addr32 = if machine.has_address_size_prefix {
        let addr32 = machine.read_u32(machine.registers.cs(), machine.registers.ip());
        machine.registers.step(Some(4));
        addr32
    } else {
        let addr16 = machine.read_u16(machine.registers.cs(), machine.registers.ip());
        machine.registers.step(Some(2));
        addr16 as u32
    };
    bytes.extend_from_slice(&addr32.to_le_bytes());
    
    // Определяем сегмент с учётом префикса
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let offset = (addr32 & 0xFFFF) as u16; // В реальном режиме смещение усекается до 16 бит
    
    // Записываем значение EAX в память по абсолютному адресу [segment:offset]
    machine.write_u32(segment, offset, machine.registers.eax());
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_eax_data(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(prev);
    let data = machine.read_u32(machine.registers.cs(), machine.registers.ip());
    bytes.extend_from_slice(&data.to_le_bytes());
    machine.log_instruction(csip, &bytes).ok();
    machine.registers.step(Some(4));
    machine.registers.set_eax(data);
}

pub fn mov_edx_data(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(prev);
    let data = machine.read_u32(machine.registers.cs(), machine.registers.ip());
    bytes.extend_from_slice(&data.to_le_bytes());
    machine.log_instruction(csip, &bytes).ok();
    machine.registers.step(Some(4));
    machine.registers.set_edx(data);
}

pub fn mov_ebx_data(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let data = machine.read_u32(machine.registers.cs(), machine.registers.ip());
    bytes.extend_from_slice(&data.to_le_bytes());
    machine.log_instruction(csip, &bytes).ok();
    machine.registers.step(Some(4));
    machine.registers.set_ebx(data);
}

pub fn mov_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);
    let src_val = machine.read_reg32(modrm.reg_field); // источник — регистр

    if modrm.is_register_mode() {
        // MOV reg32, reg32
        machine.write_reg32(modrm.rm_field, src_val);
    } else {
        // MOV [addr], reg32
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap(); // resolve_address уже обрабатывает seg override и BP→SS
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.write_phys_u32(addr, src_val);
    }

    machine.log_instruction(csip, &bytes).ok();
}

// mov32.rs
pub fn mov_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u32(addr)
    };

    machine.write_reg32(modrm.reg_field, src_val);
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov32.rs
pub fn mov_rm32_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Проверка подоперации: только /0 допустимо для опкода 0xC7
    if modrm.reg_field != 0 {
        log::error!("Invalid reg_field {} for opcode 0xC7", modrm.reg_field);
        machine.halted = true;
        return;
    }
    
    // Чтение непосредственного значения
    let imm32 = machine.read_u32(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(4));
    bytes.extend_from_slice(&imm32.to_le_bytes());
    
    if modrm.is_register_mode() {
        // MOV reg32, imm32
        machine.write_reg32(modrm.rm_field, imm32);
    } else {
        // MOV [addr], imm32
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.write_phys_u32(addr, imm32);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov32.rs
pub fn mov_esi_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm32 = machine.read_u32(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(4)); // продвигаем на 4 байта (imm32)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm32.to_le_bytes());
    
    // Устанавливаем значение в ESI
    machine.registers.set_esi(imm32);
    
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov32.rs
pub fn mov_edi_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm32 = machine.read_u32(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(4)); // продвигаем на 4 байта (imm32)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm32.to_le_bytes());
    
    // Устанавливаем значение в EDI
    machine.registers.set_edi(imm32);
    
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov32.rs
pub fn mov_eax_address32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    
    // Читаем 32-битное смещение из [CS:IP]
    let addr32 = if machine.has_address_size_prefix {
        let addr32 = machine.read_u32(machine.registers.cs(), machine.registers.ip());
        machine.registers.step(Some(4));
        addr32
    } else {
        let addr16 = machine.read_u16(machine.registers.cs(), machine.registers.ip());
        machine.registers.step(Some(2));
        addr16 as u32
    };
    bytes.extend_from_slice(&addr32.to_le_bytes());
    
    // Определяем сегмент с учётом префикса
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let offset = (addr32 & 0xFFFF) as u16; // В реальном режиме смещение усекается до 16 бит
    
    // Читаем значение из памяти по абсолютному адресу [segment:offset]
    let value = machine.read_u32(segment, offset);
    
    // Устанавливаем значение в EAX
    machine.registers.set_eax(value);
    
    machine.log_instruction(csip, &bytes).ok();
}

/// LEA r32, r/m32 — Load Effective Address (32-bit)
/// Поддерживает расширенную адресацию: [base + index*scale + disp]
pub fn lea_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    
    // Для 32-битного режима требуется SIB байт при определённых комбинациях
    let sib_byte = if (modrm_byte & 0x07) == 4 && ((modrm_byte >> 6) & 0x03) != 3 {
        let sib = machine.read_u8(machine.registers.cs(), machine.registers.ip());
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
            // При rm=4 требуется SIB байт для определения базы
            if let Some(sib) = sib_byte {
                let base_reg = sib & 0x07;
                match base_reg {
                    5 => {
                        if mod_field == 0 {
                            // [disp32] без базы
                            let disp32 = machine.read_u32(machine.registers.cs(), machine.registers.ip());
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
                // [disp32] без базы
                let disp32 = machine.read_u32(machine.registers.cs(), machine.registers.ip());
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
    
    // Дисплейсмент
    let displacement = match mod_field {
        0 => 0,
        1 => {
            let disp8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8 as i32;
            machine.registers.step(None);
            bytes.push(disp8 as u8);
            disp8 as u32
        }
        2 => {
            let disp32 = machine.read_u32(machine.registers.cs(), machine.registers.ip());
            machine.registers.step(Some(4));
            bytes.extend_from_slice(&disp32.to_le_bytes());
            disp32
        }
        _ => 0,
    };
    
    base.wrapping_add(index_scaled).wrapping_add(displacement)
}

// libs/dos_core/src/instructions/mov32.rs
pub fn mov_ebp_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    
    // Читаем 32-битное непосредственное значение (little-endian)
    let imm32 = machine.read_u32(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(4)); // продвигаем на 4 байта (imm32)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm32.to_le_bytes());
    
    // Загружаем значение в регистр EBP
    machine.registers.set_ebp(imm32);
    
    machine.log_instruction(csip, &bytes).ok();
}