// Ver: 1
use crate::{flags, machine::DosMachine, modrm::ModRm};

pub fn call(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let rel16 = machine.read_u16(machine.registers.cs(), machine.registers.ip()) as i16;
    bytes.extend_from_slice(&rel16.to_le_bytes());
    let _ = machine.log_instruction(csip, &bytes);
    let return_ip = machine.registers.ip().wrapping_add(2);

    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(machine.registers.ss(), machine.registers.sp(), return_ip);
    let new_ip = (return_ip as i32 + rel16 as i32) as u16;
    machine.registers.set_ip(new_ip);
}

pub fn retn(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let _ = machine.log_instruction(csip, prev);
    let ip = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_ip(ip);
}

pub fn jz(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    bytes.push(rel8 as u8);
    let _ = machine.log_instruction(csip, &bytes);
    machine.registers.step(None);

    if (machine.registers.flags() & (flags::ZF)) != 0 {
        let current_ip = machine.registers.ip() as i32;
        let new_ip_32 = current_ip.wrapping_add(rel8 as i32);
        let new_ip = (new_ip_32 & 0xFFFF) as u16;
        machine.registers.set_ip(new_ip);
    }
}

pub fn ja(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);
    machine.log_instruction(csip, &bytes).ok();

    let flags = machine.registers.flags();
    let cf = (flags & 0x0001) != 0;
    let zf = (flags & 0x0040) != 0;

    if !cf && !zf {
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel8 as i32) as u16;
        machine.registers.set_ip(new_ip);
    }
}

pub fn call_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    let target_ip = if modrm.is_register_mode() {
        // CALL reg16 — читаем значение из регистра
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .expect("Failed to resolve memory address in CALL r/m16");
        
        machine.read_phys_u16(addr)
    };
    
    let current_ip = machine.registers.ip();
    machine.registers.set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(machine.registers.ss(), machine.registers.sp(), current_ip);
    
    machine.registers.set_ip(target_ip);
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn smsw(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);
    if modrm.is_register_mode() {
        machine.write_reg16(modrm.rm_field, 0x0000);
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.write_phys_u16(addr, 0x0000);
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub fn jmp_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);
    let target_ip = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.read_phys_u16(addr)
    };

    machine.registers.set_ip(target_ip);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn jb(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    let cf = (machine.registers.flags() & 0x0001) != 0;
    machine.log_instruction(csip, &bytes).ok();

    if cf {
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel8 as i32) as u16;
        machine.registers.set_ip(new_ip);
    }
}

// libs/dos_core/src/instructions/control.rs
pub fn loop_cx(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (rel8)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);
    if machine.registers.cx() != 0 {
        // Уменьшаем CX на 1 (беззнаковое вычитание с wrap-around)
        let cx = machine.registers.cx().wrapping_sub(1);
        machine.registers.set_cx(cx);
        /*println!("{cx}");
        machine.log_instruction(csip, &bytes).ok();
        machine.halted = true;
        return;*/
        // Если CX ≠ 0 — выполняем переход

        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel8 as i32) as u16;
        machine.registers.set_ip(new_ip);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub fn jmp_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    // Читаем 8-битное смещение со знаком
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    // Вычисляем новый IP: текущий IP (после чтения смещения) + sign_extend(rel8)
    // В реальном режиме усекаем до 16 бит
    let new_ip = (machine.registers.ip() as i16).wrapping_add(rel8 as i16) as u16;
    machine.registers.set_ip(new_ip);

    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/control.rs
pub fn jne_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    // Читаем 8-битное смещение со знаком
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    // Проверяем флаг нуля ZF (бит 6) — прыжок при ZF=0
    let zf = (machine.registers.flags() & (flags::ZF)) != 0;

    if !zf {
        // Вычисляем новый IP: текущий IP (после чтения смещения) + sign_extend(rel8)
        // В реальном режиме усекаем до 16 бит
        let new_ip = (machine.registers.ip() as i16).wrapping_add(rel8 as i16) as u16;
        machine.registers.set_ip(new_ip);
    }

    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/control.rs
pub fn jae_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    // Читаем 8-битное смещение со знаком
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    // Проверяем флаг переноса CF (бит 0) — прыжок при CF=0
    let cf = (machine.registers.flags() & 1) != 0;

    if !cf {
        // Вычисляем новый IP: текущий IP (после чтения смещения) + sign_extend(rel8)
        // В реальном режиме усекаем до 16 бит
        let new_ip = (machine.registers.ip() as i16).wrapping_add(rel8 as i16) as u16;
        machine.registers.set_ip(new_ip);
    }

    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/control.rs
pub fn jge_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    // Читаем 8-битное смещение со знаком
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    // Проверяем флаги знака (SF) и переполнения (OF)
    // SF = бит 7, OF = бит 11
    let flags = machine.registers.flags();
    let sf = (flags & (flags::SF)) != 0; // Sign Flag (бит 7)
    let of = (flags & (flags::OF)) != 0; // Overflow Flag (бит 11)

    // Прыжок выполняется если SF == OF (результат >= 0 при знаковом сравнении)
    if sf == of {
        // Вычисляем новый IP: текущий IP (после чтения смещения) + sign_extend(rel8)
        // В реальном режиме усекаем до 16 бит
        let new_ip = (machine.registers.ip() as i16).wrapping_add(rel8 as i16) as u16;
        machine.registers.set_ip(new_ip);
    }

    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/control.rs
pub fn jmp_rel16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    // Читаем 16-битное смещение со знаком (little-endian)
    let rel16 = machine.read_u16(machine.registers.cs(), machine.registers.ip()) as i16;
    machine.registers.step(Some(2)); // продвигаем на 2 байта (смещение)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&rel16.to_le_bytes());

    // Вычисляем новый IP: текущий IP (после чтения смещения) + sign_extend(rel16)
    // В реальном режиме усекаем до 16 бит
    let new_ip = (machine.registers.ip() as i32).wrapping_add(rel16 as i32) as u16;
    machine.registers.set_ip(new_ip);

    machine.log_instruction(csip, &bytes).ok();
}

pub fn jcxz_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    // Читаем 8-битное смещение со знаком
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    // Проверяем регистр CX (НЕ флаги!)
    if machine.registers.cx() == 0 {
        // Вычисляем новый IP: текущий IP (после чтения смещения) + sign_extend(rel8)
        // В реальном режиме усекаем до 16 бит
        let new_ip = (machine.registers.ip() as i16).wrapping_add(rel8 as i16) as u16;
        machine.registers.set_ip(new_ip);
    }

    // Флаги НЕ изменяются — критически важно!
    machine.log_instruction(csip, &bytes).ok();
}

pub fn loopnz_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    // Читаем 8-битное смещение со знаком
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    // 1. Декрементируем CX (флаги НЕ устанавливаются!)
    let cx = machine.registers.cx();
    let new_cx = cx.wrapping_sub(1);
    machine.registers.set_cx(new_cx);

    // 2. Проверяем флаг нуля ZF (бит 6)
    let zf = (machine.registers.flags() & (flags::ZF)) != 0;

    // 3. Выполняем прыжок если CX ≠ 0 И ZF = 0 (не равно)
    if new_cx != 0 && !zf {
        // Вычисляем новый IP: текущий IP (после чтения смещения) + sign_extend(rel8)
        let new_ip = (machine.registers.ip() as i16).wrapping_add(rel8 as i16) as u16;
        machine.registers.set_ip(new_ip);
    }

    // Флаги НЕ изменяются — критически важно! (декремент не устанавливает флаги)
    machine.log_instruction(csip, &bytes).ok();
}

pub fn loopz_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    // Читаем 8-битное смещение со знаком
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    // 1. Декрементируем CX (флаги НЕ устанавливаются!)
    let cx = machine.registers.cx();
    let new_cx = cx.wrapping_sub(1);
    machine.registers.set_cx(new_cx);

    // 2. Проверяем флаг нуля ZF (бит 6)
    let zf = (machine.registers.flags() & (flags::ZF)) != 0;

    // 3. Выполняем прыжок если CX ≠ 0 И ZF = 1 (равно)
    if new_cx != 0 && zf {
        // Вычисляем новый IP: текущий IP (после чтения смещения) + sign_extend(rel8)
        let new_ip = (machine.registers.ip() as i16).wrapping_add(rel8 as i16) as u16;
        machine.registers.set_ip(new_ip);
    }

    // Флаги НЕ изменяются — критически важно! (декремент не устанавливает флаги)
    machine.log_instruction(csip, &bytes).ok();
}

pub fn jl_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    // Читаем 8-битное смещение со знаком
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    // Проверяем флаги знака (SF, бит 7) и переполнения (OF, бит 11)
    let flags = machine.registers.flags();
    let sf = (flags & (flags::SF)) != 0;
    let of = (flags & (flags::OF)) != 0;

    // Условие перехода: SF ≠ OF (знаковый результат отрицательный → "меньше")
    if sf != of {
        // Вычисляем новый IP: текущий IP (после чтения смещения) + sign_extend(rel8)
        let new_ip = (machine.registers.ip() as i16).wrapping_add(rel8 as i16) as u16;
        machine.registers.set_ip(new_ip);
    }

    // Флаги НЕ изменяются — критически важно!
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/control.rs
pub fn call_far(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    bytes.push(0x9A); // опкод CALL far

    // Читаем 32 бита из кода: сначала 16 бит смещения (IP), затем 16 бит сегмента (CS)
    // Порядок байтов: [IP_lo, IP_hi, CS_lo, CS_hi] (little-endian для каждого слова)
    let ip_offset = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&ip_offset.to_le_bytes());

    let cs_segment = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&cs_segment.to_le_bytes());

    // Сохраняем текущий CS:IP в стек в порядке: сначала CS, затем IP
    // 1. Сохраняем текущий CS
    let sp = machine.registers.sp().wrapping_sub(2);
    machine.registers.set_sp(sp);
    machine.write_u16(machine.registers.ss(), sp, machine.registers.cs());

    // 2. Сохраняем текущий IP
    let sp = machine.registers.sp().wrapping_sub(2);
    machine.registers.set_sp(sp);
    machine.write_u16(machine.registers.ss(), sp, machine.registers.ip());

    // 3. Загружаем новый сегмент и смещение
    machine.registers.set_cs(cs_segment);
    machine.registers.set_ip(ip_offset);

    // Флаги НЕ изменяются — критически важно!
    machine.log_instruction(csip, &bytes).ok();
}

pub fn jns_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    // Читаем 8-битное смещение со знаком
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);
    
    // Проверяем флаг знака SF (бит 7)
    let sf = (machine.registers.flags() & (flags::SF)) != 0;
    
    // Условие перехода: SF = 0 (результат неотрицательный)
    if !sf {
        // БЕЗОПАСНОЕ сложение с усечением до 16 бит (предотвращение паники при переполнении)
        let current_ip = machine.registers.ip() as i32;
        let new_ip_32 = current_ip.wrapping_add(rel8 as i32);
        let new_ip = (new_ip_32 & 0xFFFF) as u16;
        
        machine.registers.set_ip(new_ip);
    }
    
    // Флаги НЕ изменяются — критически важно!
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/control.rs
pub fn jg_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    // Читаем 8-битное смещение со знаком
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);
    
    // Проверяем флаги:
    //   ZF (бит 6) = 0 — результат не нулевой
    //   SF (бит 7) = знак результата
    //   OF (бит 11) = знаковое переполнение
    let flags = machine.registers.flags();
    let zf = (flags & (flags::ZF)) != 0;
    let sf = (flags & (flags::SF)) != 0;
    let of = (flags & (flags::OF)) != 0;
    
    // Условие перехода: ZF = 0 AND SF = OF (знаковый результат положительный)
    if !zf && (sf == of) {
        // БЕЗОПАСНОЕ сложение с усечением до 16 бит (предотвращение паники при переполнении)
        let current_ip = machine.registers.ip() as i32;
        let new_ip_32 = current_ip.wrapping_add(rel8 as i32);
        let new_ip = (new_ip_32 & 0xFFFF) as u16;
        
        machine.registers.set_ip(new_ip);
    }
    
    // Флаги НЕ изменяются — критически важно!
    machine.log_instruction(csip, &bytes).ok();
}

pub fn jmp_far(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    
    // Читаем 32 бита из кода: сначала 16 бит смещения (IP), затем 16 бит сегмента (CS)
    // Порядок байтов: [IP_lo, IP_hi, CS_lo, CS_hi] (little-endian для каждого слова)
    let ip_offset = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&ip_offset.to_le_bytes());
    
    let cs_segment = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&cs_segment.to_le_bytes());
    
    // Загружаем новый сегмент и смещение (без сохранения старого состояния!)
    machine.registers.set_cs(cs_segment);
    machine.registers.set_ip(ip_offset);
    
    // Флаги НЕ изменяются — критически важно!
    machine.log_instruction(csip, &bytes).ok();
}

pub fn retf(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let bytes = prev.to_vec();
    
    // 1. Извлекаем IP из стека (первое слово)
    let sp = machine.registers.sp();
    let ip = machine.read_u16(machine.registers.ss(), sp);
    machine.registers.set_sp(sp.wrapping_add(2));
    
    // 2. Извлекаем CS из стека (второе слово)
    let sp = machine.registers.sp();
    let cs = machine.read_u16(machine.registers.ss(), sp);
    machine.registers.set_sp(sp.wrapping_add(2));
    
    // Устанавливаем восстановленные значения
    machine.registers.set_ip(ip);
    machine.registers.set_cs(cs);
    
    // Флаги НЕ изменяются — критически важно!
    machine.log_instruction(csip, &bytes).ok();
}

pub fn jle_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    // Читаем 8-битное смещение со знаком
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);
    
    let flags = machine.registers.flags();
    let zf = (flags & (flags::ZF)) != 0;
    let sf = (flags & (flags::SF)) != 0;
    let of = (flags & (flags::OF)) != 0;
    
    if zf || (sf != of) {
        let current_ip = machine.registers.ip() as i32;
        let new_ip_32 = current_ip.wrapping_add(rel8 as i32);
        let new_ip = (new_ip_32 & 0xFFFF) as u16;
        
        machine.registers.set_ip(new_ip);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

/// CALL ptr16:16 — Far call through memory (межсегментный вызов через память)
/// Читает 32 бита из памяти: сначала 16 бит смещения (IP), затем 16 бит сегмента (CS)
pub fn call_far_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Определяем сегмент источника с учётом префикса
    let src_segment = machine.override_segment.unwrap_or(machine.registers.ds());
    
    // Читаем адрес источника (только для памяти)
    let addr = if modrm.is_register_mode() {
        // CALL far через регистр недопустим — вызывает #UD
        log::error!("CALL far through register is undefined behavior at {:#04x}:{:#04x}",
                    machine.registers.cs(), machine.registers.ip());
        machine.halted = true;
        return;
    } else {
        modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap()
    };
    // Читаем 32 бита из памяти: сначала 16 бит смещения (IP), затем 16 бит сегмента (CS)
    // Порядок байтов: [IP_lo, IP_hi, CS_lo, CS_hi] (little-endian для каждого слова)
    let addr_u16 = addr as u16; // ← ПРИВЕДЕНИЕ к u16 для 16-битного режима
    let ip_offset = machine.read_u16(src_segment, addr_u16);
    let cs_segment = machine.read_u16(src_segment, addr_u16 + 2);
    
    // Сохраняем текущий CS:IP в стек в порядке: сначала CS, затем IP
    // 1. Сохраняем текущий CS
    let sp = machine.registers.sp().wrapping_sub(2);
    machine.registers.set_sp(sp);
    machine.write_u16(machine.registers.ss(), sp, machine.registers.cs());
    
    // 2. Сохраняем текущий IP
    let sp = machine.registers.sp().wrapping_sub(2);
    machine.registers.set_sp(sp);
    machine.write_u16(machine.registers.ss(), sp, machine.registers.ip());
    
    // 3. Загружаем новый сегмент и смещение
    machine.registers.set_cs(cs_segment);
    machine.registers.set_ip(ip_offset);
    
    // Флаги НЕ изменяются
    machine.log_instruction(csip, &bytes).ok();
}

/// JMP ptr16:16 — Far jump through memory (межсегментный переход через память)
/// Читает 32 бита из памяти: сначала 16 бит смещения (IP), затем 16 бит сегмента (CS)
pub fn jmp_far_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Определяем сегмент источника с учётом префикса
    let src_segment = machine.override_segment.unwrap_or(machine.registers.ds());
    
    // ВАЖНО: для mod=00, r/m=110 адрес операнда = disp16 (абсолютное смещение в сегменте)
    // resolve_address возвращает ТОЛЬКО смещение, НЕ физический адрес!
    let addr_offset = if modrm.is_register_mode() {
        log::error!("JMP far through register is undefined behavior at {:#04x}:{:#04x}",
                    machine.registers.cs(), machine.registers.ip());
        machine.halted = true;
        return;
    } else {
        modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap()
    };
    // bytes.extend_from_slice(&addr_offset.to_le_bytes()); // ← УДАЛИТЬ! Это вычисленное смещение, не часть инструкции
    
    // КРИТИЧЕСКИ ВАЖНО: addr_offset — это СМЕЩЕНИЕ в сегменте src_segment
    // Для чтения из памяти НЕ нужно приводить к u16 — resolve_address уже вернул u32/u16
    let addr_u16 = addr_offset as u16;
    
    // Читаем 32 бита из памяти: сначала 16 бит смещения (IP), затем 16 бит сегмента (CS)
    // ВАЖНО: читаем из СЕГМЕНТА src_segment (обычно DS), а не из CS!
    let ip_offset = machine.read_u16(src_segment, addr_u16);
    let cs_segment = machine.read_u16(src_segment, addr_u16.wrapping_add(2));
    
    // Загружаем новый сегмент и смещение (без сохранения старого состояния!)
    machine.registers.set_cs(cs_segment);
    machine.registers.set_ip(ip_offset);
    
    // Флаги НЕ изменяются
    machine.log_instruction(csip, &bytes).ok();
    
    // Отладочное логирование для диагностики
    #[cfg(debug_assertions)]
    {
        log::info!(
            "JMP far: reading from [DS={:#04x}:{:#04x}] → IP={:#04x}, CS={:#04x} → jumping to {:#04x}:{:#04x}",
            src_segment,
            addr_u16,
            ip_offset,
            cs_segment,
            cs_segment,
            ip_offset
        );
    }
}

// В libs/dos_core/src/instructions/control.rs (НЕ control32.rs!)

pub fn jz_rel16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    
    // ← Читаем 16-битное смещение (sign-extended)
    let rel16 = machine.read_u16(machine.registers.cs(), machine.registers.ip()) as i16;
    machine.registers.step(Some(2));  // ← 2 байта для rel16!
    
    bytes.extend_from_slice(&rel16.to_le_bytes());
    
    // Проверяем флаг ZF (бит 6)
    let zf = (machine.registers.flags() & (flags::ZF)) != 0;
    
    if zf {
        // Вычисляем новый IP: текущий IP + sign_extend(rel16)
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel16 as i32) as u16;
        machine.registers.set_ip(new_ip);
        log::debug!("JZ rel16: TAKEN (ZF=1), new IP={:#04x}", new_ip);
    } else {
        log::debug!("JZ rel16: NOT TAKEN (ZF=0), continue");
    }
    
    machine.log_instruction(csip, &bytes).ok();
}