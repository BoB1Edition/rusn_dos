// Ver: 4

use crate::{machine::DosMachine, modrm::ModRm};

pub fn mov_ah(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let imm = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    bytes.push(imm);
    let _ = machine.log_instruction(csip, &bytes);
    machine.registers.set_ah(imm);
    machine.registers.step(None);
}

pub fn mov_dl(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let imm = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    bytes.push(imm);
    machine.log_instruction(csip, &bytes).ok();
    machine.registers.set_dl(imm);
    machine.registers.step(None);
}

pub fn mov_ax(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let imm = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    bytes.extend_from_slice(&imm.to_le_bytes());
    machine.log_instruction(csip, &bytes).ok();
    machine.registers.set_ax(imm);
    machine.registers.step(Some(2));
}

pub fn mov_dx(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let imm = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    bytes.extend_from_slice(&imm.to_le_bytes());
    let _ = machine.log_instruction(csip, &bytes);
    machine.registers.set_dx(imm);
    machine.registers.step(Some(2));
}

pub fn mov_bx(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let imm = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    bytes.extend_from_slice(&imm.to_le_bytes());
    let _ = machine.log_instruction(csip, &bytes);
    machine.registers.set_bx(imm);
    machine.registers.step(Some(2));
}

pub fn mov_rm16_sreg(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    //if !machine.has_address_size_prefix {
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);

    let sreg_value = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u16(addr)
    };
    machine.write_reg16(modrm.rm_field, sreg_value); // приёмник: общий регистр
    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let _ = machine.log_instruction(csip, &bytes);

    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        let src_reg = modrm.reg_field; // источник
        let dst_reg = modrm.rm_field; // приёмник
        let src_val = machine.read_reg16(src_reg);
        machine.write_reg16(dst_reg, src_val);
    } else {
        // Память пока не поддерживается
        log::error!("Memory operand in MOV r/m16, r16 not supported yet");
        machine.halted = true;
    }
}

pub fn mov_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u16(addr)
    };

    machine.write_reg16(modrm.reg_field, src_val);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_al_address16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let addr16 = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&addr16.to_le_bytes());

    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let value = machine.read_u8(segment, addr16);

    machine.registers.set_al(value);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_al_address32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let addr32 = machine.read_u32(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(4));
    bytes.extend_from_slice(&addr32.to_le_bytes());

    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let offset = (addr32 & 0xFFFF) as u16; 
    let value = machine.read_u8(segment, offset); 

    machine.registers.set_al(value);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_sreg_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);
    let src_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u16(addr)
    };

    // Запись в сегментный регистр
    match modrm.reg_field {
        0 => machine.registers.set_es(src_val),
        1 => {
            // MOV CS, ... — недопустимо на x86
            log::error!("Attempt to write to CS register");
            machine.halted = true;
            return;
        }
        2 => machine.registers.set_ss(src_val),
        3 => machine.registers.set_ds(src_val),
        4 => machine.registers.set_fs(src_val),  // FS (расширение 386+)
        5 => {
            machine.registers.set_gs(src_val);
            log::warn!("GS register not implemented, ignoring write");
            // Игнорируем запись в GS (как делают многие эмуляторы для совместимости)
        }
        _ => {
            log::error!("Invalid segment register field in MOV sreg, r/m16 {}", modrm.reg_field);
            machine.halted = true;
            return;
        }
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_r8_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let src_val = if modrm.is_register_mode() {
        machine.read_reg8(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u8(addr)
    };
    machine.write_reg8(modrm.reg_field, src_val);
    
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov.rs
pub fn mov_rm16_imm16(machine: &mut DosMachine, prev: &[u8]) {
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
    let imm16 = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&imm16.to_le_bytes());
    
    if modrm.is_register_mode() {
        // MOV reg16, imm16
        machine.write_reg16(modrm.rm_field, imm16);
    } else {
        // MOV [addr], imm16
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.write_phys_u16(addr, imm16);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov.rs
pub fn mov_address_ax(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    
    // Читаем 16-битное смещение из [CS:IP]
    let addr16 = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&addr16.to_le_bytes());
    
    // Определяем сегмент с учётом префикса
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    
    // Записываем значение AX в память по абсолютному адресу [segment:addr16]
    machine.write_u16(segment, addr16, machine.registers.ax());
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_al_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm8 = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None); // продвигаем на 1 байт (imm8)
    let mut bytes = prev.to_vec();
    bytes.push(imm8);
    
    // Устанавливаем значение в AL (не изменяя остальные биты EAX)
    machine.registers.set_al(imm8);
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_bh_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm8 = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None); // продвигаем на 1 байт (imm8)
    let mut bytes = prev.to_vec();
    bytes.push(imm8);
    
    // Устанавливаем значение в BH (не изменяя остальные биты EBX)
    machine.registers.set_bh(imm8);
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_bl_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm8 = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None); // продвигаем на 1 байт (imm8)
    let mut bytes = prev.to_vec();
    bytes.push(imm8);
    
    // Устанавливаем значение в BH (не изменяя остальные биты EBX)
    machine.registers.set_bl(imm8);
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn stosw(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let bytes = prev.to_vec();
    
    // Читаем флаг направления DF (бит 10)
    let df = (machine.registers.flags() & (1 << 10)) != 0;
    
    // Записываем слово AX в [ES:DI]
    let ax = machine.registers.ax();
    machine.write_u16(machine.registers.es(), machine.registers.di(), ax);
    
    // Обновляем DI в зависимости от флага направления
    if df {
        machine.registers.set_di(machine.registers.di().wrapping_sub(2));
    } else {
        machine.registers.set_di(machine.registers.di().wrapping_add(2));
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov.rs
pub fn mov_si_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm16 = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(2)); // продвигаем на 2 байта (imm16)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());
    
    // Устанавливаем значение в SI (не изменяя остальные биты ESI)
    machine.registers.set_si(imm16);
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_di_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm16 = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(2)); // продвигаем на 2 байта (imm16)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());
    
    // Устанавливаем значение в DI (не изменяя остальные биты EDI)
    machine.registers.set_di(imm16);
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_cx_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm16 = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(2)); // продвигаем на 2 байта (imm16)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());
    
    // Устанавливаем значение в CX (не изменяя остальные биты ECX)
    machine.registers.set_cx(imm16);
    
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov.rs
/*pub fn mov_al_address16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    
    // Читаем 16-битное смещение из [CS:IP]
    let addr16 = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&addr16.to_le_bytes());
    
    // Определяем сегмент с учётом префикса
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    
    // Читаем значение из памяти по абсолютному адресу [segment:addr16]
    let value = machine.read_u8(segment, addr16);
    
    // Устанавливаем значение в AL
    machine.registers.set_al(value);
    
    machine.log_instruction(csip, &bytes).ok();
}*/

pub fn mov_ax_address16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    
    // Читаем 16-битное смещение из [CS:IP]
    let addr16 = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&addr16.to_le_bytes());
    
    // Определяем сегмент с учётом префикса
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    
    // Читаем значение из памяти по абсолютному адресу [segment:addr16]
    let value = machine.read_u16(segment, addr16);
    
    // Устанавливаем значение в AX
    machine.registers.set_ax(value);
    
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov.rs
pub fn stosb(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let bytes = prev.to_vec();
    
    // Читаем флаг направления DF (бит 10)
    let df = (machine.registers.flags() & (1 << 10)) != 0;
    
    // Записываем байт AL в [ES:DI]
    let al = machine.registers.al();
    machine.write_u8(machine.registers.es(), machine.registers.di(), al);
    
    // Обновляем DI в зависимости от флага направления
    if df {
        machine.registers.set_di(machine.registers.di().wrapping_sub(1));
    } else {
        machine.registers.set_di(machine.registers.di().wrapping_add(1));
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Источник: регистр из reg_field
    let src_val = machine.read_reg8(modrm.reg_field);
    
    // Приёмник: r/m8 (регистр или память)
    if modrm.is_register_mode() {
        // MOV reg8, reg8
        machine.write_reg8(modrm.rm_field, src_val);
    } else {
        // MOV [mem], reg8
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.write_phys_u8(addr, src_val);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov.rs
pub fn lodsb(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let bytes = prev.to_vec();
    
    // Определяем сегмент с учётом префикса
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    
    // Читаем флаг направления DF (бит 10)
    let df = (machine.registers.flags() & (1 << 10)) != 0;
    
    // Загружаем байт из [segment:SI] в AL
    let si = machine.registers.si();
    let al = machine.read_u8(segment, si);
    machine.registers.set_al(al);
    
    // Обновляем SI в зависимости от флага направления
    if df {
        machine.registers.set_si(si.wrapping_sub(1));
    } else {
        machine.registers.set_si(si.wrapping_add(1));
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov.rs
pub fn mov_rm8_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    
    // Читаем байт ModR/M
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // ⚠️ КРИТИЧЕСКАЯ ПРОВЕРКА: только reg_field = 0 допустимо для 0xC6
    if modrm.reg_field != 0 {
        log::error!(
            "Invalid reg_field {} in MOV r/m8, imm8 (opcode 0xC6). Only /0 is valid.",
            modrm.reg_field
        );
        machine.halted = true;
        return;
    }
    
    // Читаем непосредственное значение imm8
    let imm8 = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    bytes.push(imm8);
    
    // Записываем значение в приёмник (регистр или память)
    if modrm.is_register_mode() {
        // MOV reg8, imm8
        machine.write_reg8(modrm.rm_field, imm8);
    } else {
        // MOV [mem], imm8
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.write_phys_u8(addr, imm8);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}