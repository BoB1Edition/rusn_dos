// Ver: 2

use crate::{flags, machine::DosMachine, modrm::ModRm};

pub fn mov_ah(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let imm = machine.read_instr_u8( machine.registers.ip());
    bytes.push(imm);
    let _ = machine.log_instruction(csip, &bytes);
    machine.registers.set_ah(imm);
    machine.registers.step(None);
}

pub fn mov_dl(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let imm = machine.read_instr_u8( machine.registers.ip());
    bytes.push(imm);
    machine.log_instruction(csip, &bytes).ok();
    machine.registers.set_dl(imm);
    machine.registers.step(None);
}

pub fn mov_ax(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let imm = machine.read_instr_u16(machine.registers.ip());
    bytes.extend_from_slice(&imm.to_le_bytes());
    machine.log_instruction(csip, &bytes).ok();
    machine.registers.set_ax(imm);
    machine.registers.step(Some(2));
}

pub fn mov_dx(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let imm = machine.read_instr_u16( machine.registers.ip());
    bytes.extend_from_slice(&imm.to_le_bytes());
    let _ = machine.log_instruction(csip, &bytes);
    machine.registers.set_dx(imm);
    machine.registers.step(Some(2));
}

pub fn mov_bx(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let imm = machine.read_instr_u16( machine.registers.ip());
    bytes.extend_from_slice(&imm.to_le_bytes());
    let _ = machine.log_instruction(csip, &bytes);
    machine.registers.set_bx(imm);
    machine.registers.step(Some(2));
}

/*pub fn mov_rm16_sreg(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();
    //if !machine.has_address_size_prefix {
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);

    let sreg_value = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.read_phys_u16(addr)
    };
    machine.write_reg16(modrm.rm_field, sreg_value); // приёмник: общий регистр
    machine.log_instruction(csip, &bytes).ok();
}*/

pub fn mov_rm16_sreg(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    // ← КРИТИЧЕСКИ ВАЖНО: источник — СЕГМЕНТНЫЙ регистр из reg_field
    let sreg_value = match modrm.reg_field {
        0 => machine.registers.es(),
        1 => machine.registers.cs(),
        2 => machine.registers.ss(),
        3 => machine.registers.ds(),
        4 => machine.registers.fs(),
        5 => machine.registers.gs(),
        _ => unreachable!(),
    };

    // Приёмник — регистр или память из rm_field
    if modrm.is_register_mode() {
        machine.write_reg16(modrm.rm_field, sreg_value);
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.write_phys_u16(addr, sreg_value);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.cs());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    // Источник: регистр из поля reg_field
    let src_val = machine.read_reg16(modrm.reg_field);

    // Приёмник: регистр или память из поля rm_field
    if modrm.is_register_mode() {
        // MOV reg16, reg16
        machine.write_reg16(modrm.rm_field, src_val);
    } else {
        // MOV [mem], reg16 — ПОЛНАЯ ПОДДЕРЖКА ПАМЯТИ!
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .expect("Failed to resolve memory address in MOV r/m16, r16");

        // Для отладки: проверяем корректность адреса в реальном режиме
        #[cfg(debug_assertions)]
        {
            let linear_addr = (machine.registers.ds() as u32 * 16 + addr as u32) & 0xFFFFF;
            if linear_addr >= 0x100000 {
                log::warn!(
                    "MOV to memory beyond 1MB: DS={:#04x}, offset={:#04x} → linear={:#06x}",
                    machine.registers.ds(),
                    addr,
                    linear_addr
                );
            }
        }

        machine.write_phys_u16(addr, src_val);
    }

    // Логируем ПОСЛЕ полного формирования байтов (включая дисплейсмент для памяти)
    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
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
        machine.read_phys_u16(addr)
    };

    machine.write_reg16(modrm.reg_field, src_val);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_al_address16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let addr16 = machine.read_instr_u16( machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&addr16.to_le_bytes());

    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let value = machine.read_u8(segment, addr16);
    let phys_addr = ((segment as u32) << 4).wrapping_add(addr16 as u32);
    machine.registers.set_al(value);
    log::trace!(
        "MOV AL, [addr]: segment={:#04x} (override={:?}), offset={:#04x}, phys={:#06x}",
        segment,
        machine.override_segment,
        addr16,
        phys_addr
    );
    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_al_address32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let addr32 = machine.read_instr_u32( machine.registers.ip());
    machine.registers.step(Some(4));
    bytes.extend_from_slice(&addr32.to_le_bytes());

    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let offset = (addr32 & 0xFFFF) as u16;
    let value = machine.read_u8(segment, offset);

    machine.registers.set_al(value);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_sreg_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
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
        4 => machine.registers.set_fs(src_val), // FS (расширение 386+)
        5 => {
            machine.registers.set_gs(src_val);
        }
        _ => {
            log::error!(
                "Invalid segment register field in MOV sreg, r/m16 {}",
                modrm.reg_field
            );
            machine.halted = true;
            return;
        }
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_r8_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
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
        machine.read_phys_u8(addr)
    };
    machine.write_reg8(modrm.reg_field, src_val);

    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov.rs
pub fn mov_rm16_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
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
    let imm16 = machine.read_instr_u16( machine.registers.ip());
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
        machine.write_phys_u16(addr, imm16);
    }

    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov.rs
pub fn mov_address_ax(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();

    // Читаем 16-битное смещение из [CS:IP]
    let addr16 = machine.read_instr_u16( machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&addr16.to_le_bytes());

    // Определяем сегмент с учётом префикса
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());

    // Записываем значение AX в память по абсолютному адресу [segment:addr16]
    machine.write_u16(segment, addr16, machine.registers.ax());
    let phys = ((segment as u32) << 4).wrapping_add(addr16 as u32);
    log::debug!(
        "MOV [addr], AX: seg={:#04x}, offset={:#04x}, phys={:#06x}, a20={}",
        segment,
        addr16,
        phys,
        machine.a20_enabled // ← УЛУЧШЕННОЕ ЛОГИРОВАНИЕ
    );
    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_al_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let imm8 = machine.read_instr_u8( machine.registers.ip());
    machine.registers.step(None); // продвигаем на 1 байт (imm8)
    let mut bytes = prev.to_vec();
    bytes.push(imm8);

    // Устанавливаем значение в AL (не изменяя остальные биты EAX)
    machine.registers.set_al(imm8);

    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_bh_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let imm8 = machine.read_instr_u8( machine.registers.ip());
    machine.registers.step(None); // продвигаем на 1 байт (imm8)
    let mut bytes = prev.to_vec();
    bytes.push(imm8);

    // Устанавливаем значение в BH (не изменяя остальные биты EBX)
    machine.registers.set_bh(imm8);

    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_bl_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let imm8 = machine.read_instr_u8( machine.registers.ip());
    machine.registers.step(None); // продвигаем на 1 байт (imm8)
    let mut bytes = prev.to_vec();
    bytes.push(imm8);

    // Устанавливаем значение в BH (не изменяя остальные биты EBX)
    machine.registers.set_bl(imm8);

    machine.log_instruction(csip, &bytes).ok();
}

pub fn stosw(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let bytes = prev.to_vec();

    // Читаем флаг направления DF (бит 10)
    let df = (machine.registers.flags() & (flags::DF)) != 0;

    // Записываем слово AX в [ES:DI]
    let ax = machine.registers.ax();
    machine.write_u16(machine.registers.es(), machine.registers.di(), ax);

    // Обновляем DI в зависимости от флага направления
    if df {
        machine
            .registers
            .set_di(machine.registers.di().wrapping_sub(2));
    } else {
        machine
            .registers
            .set_di(machine.registers.di().wrapping_add(2));
    }

    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov.rs
pub fn mov_si_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let imm16 = machine.read_instr_u16( machine.registers.ip());
    machine.registers.step(Some(2)); // продвигаем на 2 байта (imm16)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());

    // Устанавливаем значение в SI (не изменяя остальные биты ESI)
    machine.registers.set_si(imm16);

    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_di_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let imm16 = machine.read_instr_u16( machine.registers.ip());
    machine.registers.step(Some(2)); // продвигаем на 2 байта (imm16)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());

    // Устанавливаем значение в DI (не изменяя остальные биты EDI)
    machine.registers.set_di(imm16);

    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_cx_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let imm16 = machine.read_instr_u16( machine.registers.ip());
    machine.registers.step(Some(2)); // продвигаем на 2 байта (imm16)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());

    // Устанавливаем значение в CX (не изменяя остальные биты ECX)
    machine.registers.set_cx(imm16);

    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_ax_address16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let mut bytes = prev.to_vec();

    // Определяем сегмент с учётом префикса
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());

    // Читаем адрес в зависимости от префикса адресации (0x67)
    let addr = if machine.has_address_size_prefix {
        // 32-битный режим адресации: читаем 32 бита, но в реальном режиме используем только младшие 16 бит
        let addr32 = machine.read_instr_u32(machine.registers.ip());
        machine.registers.step(Some(4));
        bytes.extend_from_slice(&addr32.to_le_bytes());

        // В реальном режиме 32-битное смещение усекается до 16 бит
        (addr32 & 0xFFFF) as u16
    } else {
        // 16-битный режим адресации: читаем 16 бит
        let addr16 = machine.read_instr_u16( machine.registers.ip());
        machine.registers.step(Some(2));
        bytes.extend_from_slice(&addr16.to_le_bytes());
        addr16
    };

    // Читаем значение из памяти по адресу [segment:addr]
    let value = machine.read_u16(segment, addr);

    // Устанавливаем значение в AX
    machine.registers.set_ax(value);

    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov.rs
pub fn stosb(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let bytes = prev.to_vec();

    // Читаем флаг направления DF (бит 10)
    let df = (machine.registers.flags() & (flags::DF)) != 0;

    // Записываем байт AL в [ES:DI]
    let al = machine.registers.al();
    machine.write_u8(machine.registers.es(), machine.registers.di(), al);

    // Обновляем DI в зависимости от флага направления
    if df {
        machine
            .registers
            .set_di(machine.registers.di().wrapping_sub(1));
    } else {
        machine
            .registers
            .set_di(machine.registers.di().wrapping_add(1));
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
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
        machine.write_phys_u8(addr, src_val);
    }

    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov.rs
pub fn lodsb(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let bytes = prev.to_vec();

    // Определяем сегмент с учётом префикса
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());

    // Читаем флаг направления DF (бит 10)
    let df = (machine.registers.flags() & (flags::DF)) != 0;

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
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();

    // Читаем байт ModR/M
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
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
    let imm8 = machine.read_instr_u8( machine.registers.ip());
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
        machine.write_phys_u8(addr, imm8);
    }

    machine.log_instruction(csip, &bytes).ok();
}

/// LEA r16, r/m16 — Load Effective Address (16-bit)
/// Вычисляет эффективный адрес операнда r/m и загружает его в регистр
/// ВАЖНО: НЕ читает из памяти, только вычисляет адрес!
pub fn lea_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    // LEA с режимом регистра-регистра (mod=11) технически недопустим по спецификации
    // На реальных процессорах ведёт себя как MOV. Эмулируем это поведение для совместимости.
    if modrm.is_register_mode() {
        log::warn!(
            "LEA with register mode (mod=11) at {:#04x}:{:#04x} — emulating as MOV",
            machine.registers.cs(),
            machine.registers.ip()
        );
        let src_val = machine.read_reg16(modrm.rm_field);
        machine.write_reg16(modrm.reg_field, src_val);
    } else {
        // Вычисляем эффективный адрес БЕЗ применения сегмента
        // Возвращаем только 16-битное смещение (без базы сегмента × 16)
        let offset = compute_lea_offset_16(machine, &modrm, &mut bytes);
        machine.write_reg16(modrm.reg_field, offset);
    }

    // Флаги НЕ изменяются — критически важно!
    machine.log_instruction(csip, &bytes).ok();
}

/// Вспомогательная функция: вычисление 16-битного эффективного смещения
/// Соответствует таблице адресации реального режима:
///   00 000 = [BX+SI], 00 001 = [BX+DI], 00 010 = [BP+SI], 00 011 = [BP+DI]
///   00 100 = [SI],    00 101 = [DI],    00 110 = disp16,   00 111 = [BX]
///   01 xxx = [reg+disp8], 10 xxx = [reg+disp16]
fn compute_lea_offset_16(machine: &mut DosMachine, modrm: &ModRm, bytes: &mut Vec<u8>) -> u16 {
    let mod_field = modrm.mod_field; // ← уже 0-3
    let rm_field = modrm.rm_field; // ← уже 0-7

    // Определяем базовый адрес на основе поля rm_field и mod_field
    let base = match (mod_field, rm_field) {
        (0, 0) => machine.registers.bx() as i32 + machine.registers.si() as i32,
        (0, 1) => machine.registers.bx() as i32 + machine.registers.di() as i32,
        (0, 2) => machine.registers.bp() as i32 + machine.registers.si() as i32,
        (0, 3) => machine.registers.bp() as i32 + machine.registers.di() as i32,
        (0, 4) => machine.registers.si() as i32,
        (0, 5) => machine.registers.di() as i32,
        (0, 6) => {
            // disp16 (прямой 16-битный адрес)
            let disp16 = machine.read_instr_u16( machine.registers.ip()) as i32;
            machine.registers.step(Some(2));
            bytes.extend_from_slice(&disp16.to_le_bytes());
            disp16
        }
        (0, 7) => machine.registers.bx() as i32,
        (1, 0) => machine.registers.bx() as i32 + machine.registers.si() as i32,
        (1, 1) => machine.registers.bx() as i32 + machine.registers.di() as i32,
        (1, 2) => machine.registers.bp() as i32 + machine.registers.si() as i32,
        (1, 3) => machine.registers.bp() as i32 + machine.registers.di() as i32,
        (1, 4) => machine.registers.si() as i32,
        (1, 5) => machine.registers.di() as i32,
        (1, 6) => machine.registers.bp() as i32,
        (1, 7) => machine.registers.bx() as i32,
        (2, 0) => machine.registers.bx() as i32 + machine.registers.si() as i32,
        (2, 1) => machine.registers.bx() as i32 + machine.registers.di() as i32,
        (2, 2) => machine.registers.bp() as i32 + machine.registers.si() as i32,
        (2, 3) => machine.registers.bp() as i32 + machine.registers.di() as i32,
        (2, 4) => machine.registers.si() as i32,
        (2, 5) => machine.registers.di() as i32,
        (2, 6) => machine.registers.bp() as i32,
        (2, 7) => machine.registers.bx() as i32,
        _ => unreachable!(),
    };

    // Добавляем дисплейсмент в зависимости от mod_field
    let displacement = match mod_field {
        0 => 0, // mod=00: только для rm=6 есть disp16 (обработано выше)
        1 => {
            // mod=01: disp8 (sign-extended)
            let disp8 =
                machine.read_instr_u8( machine.registers.ip()) as i8 as i32;
            machine.registers.step(None);
            bytes.push(disp8 as u8);
            disp8
        }
        2 => {
            // mod=10: disp16
            let disp16 = machine.read_instr_u16( machine.registers.ip()) as i32;
            machine.registers.step(Some(2));
            bytes.extend_from_slice(&disp16.to_le_bytes());
            disp16
        }
        _ => 0,
    };

    // Усекаем до 16 бит (как в реальном режиме)
    ((base + displacement) & 0xFFFF) as u16
}

// libs/dos_core/src/instructions/mov.rs
pub fn cmpsb(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();
    bytes.push(0xA6); // опкод CMPSB

    // Определяем сегмент источника с учётом префикса
    let src_segment = machine.override_segment.unwrap_or(machine.registers.ds());

    // Читаем байты из обоих источников
    let si = machine.registers.si();
    let di = machine.registers.di();
    let src_byte = machine.read_u8(src_segment, si);
    let dst_byte = machine.read_u8(machine.registers.es(), di);

    // Вычисляем флаги как при вычитании (беззнаковое и знаковое)
    let result = src_byte.wrapping_sub(dst_byte);

    // Флаг переноса (CF): 1 если беззнаковое переполнение (src < dst)
    let cf = src_byte < dst_byte;

    // Флаг переполнения (OF): знаковое переполнение при вычитании
    let src_sign = (src_byte as i8) < 0;
    let dst_sign = (dst_byte as i8) < 0;
    let result_sign = (result as i8) < 0;
    let of = (src_sign != dst_sign) && (src_sign != result_sign);

    // Флаг вспомогательного переноса (AF): перенос из бита 3 в бит 4
    let af = ((src_byte & 0x0F) as i8) < ((dst_byte & 0x0F) as i8);

    // Устанавливаем флаги
    machine
        .registers
        .set_flags(flags::compute_flags_u8(machine.registers.flags(), result, cf, of, af));

    // Обновляем указатели в зависимости от флага направления DF (бит 10)
    let df = (machine.registers.flags() & (flags::DF)) != 0;
    if df {
        machine.registers.set_si(si.wrapping_sub(1));
        machine.registers.set_di(di.wrapping_sub(1));
    } else {
        machine.registers.set_si(si.wrapping_add(1));
        machine.registers.set_di(di.wrapping_add(1));
    }

    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov.rs
pub fn cmpsw(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();
    bytes.push(0xA7); // опкод CMPSW

    // Определяем сегмент источника с учётом префикса
    let src_segment = machine.override_segment.unwrap_or(machine.registers.ds());

    // Читаем слова из обоих источников
    let si = machine.registers.si();
    let di = machine.registers.di();
    let src_word = machine.read_u16(src_segment, si);
    let dst_word = machine.read_u16(machine.registers.es(), di);

    // Вычисляем флаги как при вычитании (беззнаковое и знаковое)
    let result = src_word.wrapping_sub(dst_word);

    // Флаг переноса (CF): 1 если беззнаковое переполнение (src < dst)
    let cf = src_word < dst_word;

    // Флаг переполнения (OF): знаковое переполнение при вычитании
    let src_sign = (src_word as i16) < 0;
    let dst_sign = (dst_word as i16) < 0;
    let result_sign = (result as i16) < 0;
    let of = (src_sign != dst_sign) && (src_sign != result_sign);

    // Флаг вспомогательного переноса (AF): перенос из бита 3 в бит 4
    let af = ((src_word & 0x0F) as i16) < ((dst_word & 0x0F) as i16);

    // Устанавливаем флаги
    machine
        .registers
        .set_flags(flags::compute_flags_u16(machine.registers.flags(), result, cf, of, af));

    // Обновляем указатели в зависимости от флага направления DF (бит 10)
    let df = (machine.registers.flags() & (flags::DF)) != 0;
    if df {
        machine.registers.set_si(si.wrapping_sub(2));
        machine.registers.set_di(di.wrapping_sub(2));
    } else {
        machine.registers.set_si(si.wrapping_add(2));
        machine.registers.set_di(di.wrapping_add(2));
    }

    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/mov.rs
pub fn movsb(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();
    bytes.push(0xA4); // опкод MOVSB

    // Определяем сегмент источника с учётом префикса
    let src_segment = machine.override_segment.unwrap_or(machine.registers.ds());

    // Читаем байт из источника [DS:SI]
    let si = machine.registers.si();
    let byte = machine.read_u8(src_segment, si);

    // Записываем байт в приёмник [ES:DI] (сегмент ES фиксирован)
    let di = machine.registers.di();
    machine.write_u8(machine.registers.es(), di, byte);

    // Обновляем указатели в зависимости от флага направления DF (бит 10)
    let df = (machine.registers.flags() & (flags::DF)) != 0;
    if df {
        machine.registers.set_si(si.wrapping_sub(1));
        machine.registers.set_di(di.wrapping_sub(1));
    } else {
        machine.registers.set_si(si.wrapping_add(1));
        machine.registers.set_di(di.wrapping_add(1));
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_bp_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];

    // Читаем 16-битное непосредственное значение (little-endian)
    let imm16 = machine.read_instr_u16( machine.registers.ip());
    machine.registers.step(Some(2)); // продвигаем на 2 байта (imm16)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());

    // Загружаем значение в регистр BP
    machine.registers.set_bp(imm16);

    machine.log_instruction(csip, &bytes).ok();
}

pub fn mov_ax_address32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();
    
    // Читаем 32-битное смещение из потока инструкций
    let addr32 = machine.read_instr_u32( machine.registers.ip());
    machine.registers.step(Some(4));
    bytes.extend_from_slice(&addr32.to_le_bytes());
    
    // Определяем сегмент с учётом префикса override
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    
    // В реальном режиме 32-битный адрес усекается до 16 бит перед применением сегмента
    // Это поведение совместимо с 80386+ в real mode
    let offset = (addr32 & 0xFFFF) as u16;
    
    // Читаем 16-битное значение из памяти (без префикса 0x66 = 16 бит)
    let value = machine.read_u16(segment, offset);
    
    // Записываем в AX (не EAX!)
    machine.registers.set_ax(value);
    
    let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
    log::trace!(
        "MOV AX, [addr32]: segment={:#04x}, offset32={:#08x}, offset16={:#04x}, phys={:#06x}, value={:#04x}",
        segment, addr32, offset, phys_addr, value
    );
    
    machine.log_instruction(csip, &bytes).ok();
}