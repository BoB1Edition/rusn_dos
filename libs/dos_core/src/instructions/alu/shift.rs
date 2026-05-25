// Ver: 1 File: ./libs/dos_core/src/instructions/alu/shift.rs
use crate::{DosMachine, flags, modrm::ModRm};

pub fn shift_group_c1_16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
    let imm8 = machine.read_instr_u8(machine.registers.ip().wrapping_add(1));
    machine.registers.step(Some(2));
    bytes.push(modrm_byte);
    bytes.push(imm8);
    let modrm = ModRm::from_byte(modrm_byte);

    let (value, addr_opt) = if modrm.is_register_mode() {
        (machine.read_reg16(modrm.rm_field), None)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u16(addr), Some(addr))
    };

    let (result, new_flags) =
        perform_shift_16(machine,modrm.reg_field, value, imm8, machine.registers.flags());

    if let Some(addr) = addr_opt {
        // Запись обратно в память
        machine.write_phys_u16(addr, result);
    } else {
        // Запись в регистр
        machine.write_reg16(modrm.rm_field, result);
    }

    machine.registers.set_flags(new_flags);
    machine.log_instruction(csip, &bytes).ok();
}

fn perform_shift_16(machine: &DosMachine, op_field: u8, value: u16, count: u8, flags: u16) -> (u16, u16) {
    let count = count & 0x0F; // x86: count mod 16 для 16-битных операций
    if count == 0 {
        return (value, flags);
    }
    let mut new_flags = flags;
    let af = false; // AF не определён для сдвигов
    let result = match op_field {
        0 => {
            // ROL
            let result = value.rotate_left(count as u32);
            let cf = (value >> (16 - count)) & 1 != 0;
            let msb_before = (value >> 15) != 0;
            let msb_after = (result >> 15) != 0;
            let of = if count == 1 {
                msb_before != msb_after
            } else {
                false
            };
            new_flags = flags::compute_flags_u16(machine.registers.flags(), result, cf, of, af);
            result
        }
        1 => {
            // ROR
            let result = value.rotate_right(count as u32);
            let cf = (value >> (count - 1)) & 1 != 0;
            let lsb_before = (value & 1) != 0;
            let msb_after = (result >> 15) != 0;
            let of = if count == 1 {
                lsb_before != msb_after
            } else {
                false
            };
            new_flags = flags::compute_flags_u16(machine.registers.flags(), result, cf, of, af);
            result
        }
        2 => {
            // RCL
            let carry_in = (flags & 1) != 0;
            let input = (value as u32) | ((carry_in as u32) << 16);
            let rotated = input.rotate_left(count as u32);
            let result = rotated as u16;
            let new_cf = (rotated >> 16) != 0;
            let msb_before = (value >> 15) != 0;
            let msb_after = (result >> 15) != 0;
            let of = if count == 1 {
                msb_before != msb_after
            } else {
                false
            };
            new_flags = flags::compute_flags_u16(machine.registers.flags(), result, new_cf, of, af);
            result
        }
        3 => {
            // RCR
            let carry_in = (flags & 1) != 0;
            let input = (value as u32) | ((carry_in as u32) << 16);
            let rotated = input.rotate_right(count as u32);
            let result = rotated as u16;
            let new_cf = (rotated >> 16) != 0;
            let of = if count == 1 {
                let msb = (result >> 15) & 1;
                let second_msb = (result >> 14) & 1;
                msb != second_msb
            } else {
                false
            };
            new_flags = flags::compute_flags_u16(machine.registers.flags(), result, new_cf, of, af);
            result
        }
        4 | 6 => {
            // SHL / SAL (одинаково)
            let extended = value as u32;
            let shifted = extended << count;
            let result = shifted as u16;
            let cf = (shifted >> 16) != 0;
            let of = if count == 1 {
                let msb_result = (result >> 15) & 1;
                let cf_bit = cf as u16;
                (msb_result ^ cf_bit) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u16(machine.registers.flags(), result, cf, of, af);
            result
        }
        5 => {
            // SHR
            let shifted = value >> count;
            let result = shifted;
            let cf = (value >> (count - 1)) & 1 != 0;
            let of = if count == 1 {
                (((result >> 15) ^ (cf as u16)) & 1) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u16(machine.registers.flags(), result, cf, of, af);
            result
        }
        7 => {
            // SAR
            let shifted = (value as i16).wrapping_shr(count as u32) as u16;
            let result = shifted;
            let cf = if count == 0 {
                false
            } else {
                (value >> (count - 1)) & 1 != 0
            };
            let of = false; // OF cleared for SAR
            new_flags = flags::compute_flags_u16(machine.registers.flags(), result, cf, of, af);
            result
        }
        _ => unreachable!(),
    };
    (result, new_flags)
}

pub fn shift_group_d1(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
    machine.registers.step(None); // продвигаем только на байт ModR/M
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        // ROL/ROR/RCL/RCR/SHL/SHR/SAR reg16, 1
        let value = machine.read_reg16(modrm.rm_field);
        let (result, new_flags) =
            perform_shift_16(machine, modrm.reg_field, value, 1, machine.registers.flags());
        machine.write_reg16(modrm.rm_field, result);
        machine.registers.set_flags(new_flags);
    } else {
        // ROL/ROR/RCL/RCR/SHL/SHR/SAR [mem], 1
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let value = machine.read_phys_u16(addr);
        let (result, new_flags) =
            perform_shift_16(machine,modrm.reg_field, value, 1, machine.registers.flags());
        machine.write_phys_u16(addr, result);
        machine.registers.set_flags(new_flags);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub fn shift_group_c0_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
    let imm8 = machine.read_instr_u8(machine.registers.ip().wrapping_add(1));
    machine.registers.step(Some(2));
    bytes.push(modrm_byte);
    bytes.push(imm8);
    let modrm = ModRm::from_byte(modrm_byte);

    // Для 8-битных операций: счётчик сдвига по модулю 8
    let count = imm8 & 0x07;

    if count == 0 {
        // Сдвиг на 0 бит — ничего не делаем (флаги не изменяются)
        machine.log_instruction(csip, &bytes).ok();
        return;
    }

    let (value, addr_opt) = if modrm.is_register_mode() {
        (machine.read_reg8(modrm.rm_field), None)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u8(addr), Some(addr))
    };

    let (result, new_flags) =
        perform_shift_8(modrm.reg_field, value, count, machine.registers.flags());

    if let Some(addr) = addr_opt {
        // Запись обратно в память
        machine.write_phys_u8(addr, result);
    } else {
        // Запись в регистр
        machine.write_reg8(modrm.rm_field, result);
    }

    machine.registers.set_flags(new_flags);
    machine.log_instruction(csip, &bytes).ok();
}

fn perform_shift_8(op_field: u8, value: u8, count: u8, flags: u16) -> (u8, u16) {
    debug_assert!(
        count > 0 && count < 8,
        "count must be 1..7 for 8-bit shifts"
    );

    let mut new_flags = flags;
    let af = false; // AF не определён для сдвигов

    let result = match op_field {
        0 => {
            // ROL — Rotate Left
            let result = value.rotate_left(count as u32);
            let cf = (value >> (8 - count)) & 1 != 0;
            let of = if count == 1 {
                let msb_before = (value >> 7) != 0;
                let msb_after = (result >> 7) != 0;
                msb_before != msb_after
            } else {
                false
            };
            new_flags = flags::compute_flags_u8(flags, result, cf, of, af);
            result
        }
        1 => {
            // ROR — Rotate Right
            let result = value.rotate_right(count as u32);
            let cf = (value >> (count - 1)) & 1 != 0;
            let of = if count == 1 {
                let lsb_before = (value & 1) != 0;
                let msb_after = (result >> 7) != 0;
                lsb_before != msb_after
            } else {
                false
            };
            new_flags = flags::compute_flags_u8(flags, result, cf, of, af);
            result
        }
        2 => {
            // RCL — Rotate Left through Carry
            let carry_in = (flags & 1) != 0;
            let input = (value as u16) | ((carry_in as u16) << 8);
            let rotated = input.rotate_left(count as u32);
            let result = rotated as u8;
            let new_cf = (rotated >> 8) != 0;
            let of = if count == 1 {
                let msb_before = (value >> 7) != 0;
                let msb_after = (result >> 7) != 0;
                msb_before != msb_after
            } else {
                false
            };
            new_flags = flags::compute_flags_u8(flags, result, new_cf, of, af);
            result
        }
        3 => {
            // RCR — Rotate Right through Carry
            let carry_in = (flags & 1) != 0;
            let input = (value as u16) | ((carry_in as u16) << 8);
            let rotated = input.rotate_right(count as u32);
            let result = rotated as u8;
            let new_cf = (rotated >> 8) != 0;
            let of = if count == 1 {
                let msb = (result >> 7) & 1;
                let second_msb = (result >> 6) & 1;
                msb != second_msb
            } else {
                false
            };
            new_flags = flags::compute_flags_u8(flags, result, new_cf, of, af);
            result
        }
        4 | 6 => {
            // SHL / SAL — Shift Left (одинаковы)
            let extended = value as u16;
            let shifted = extended << count;
            let result = shifted as u8;
            let cf = (shifted >> 8) != 0;
            let of = if count == 1 {
                let msb_result = (result >> 7) & 1;
                let cf_bit = cf as u8;
                (msb_result ^ cf_bit) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u8(flags, result, cf, of, af);
            result
        }
        5 => {
            // SHR — Shift Right Logical
            let shifted = value >> count;
            let result = shifted;
            let cf = (value >> (count - 1)) & 1 != 0;
            let of = if count == 1 {
                let msb_result = (result >> 7) & 1;
                let cf_bit = cf as u8;
                (msb_result ^ cf_bit) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u8(flags, result, cf, of, af);
            result
        }
        7 => {
            // SAR — Shift Right Arithmetic
            let shifted = (value as i8).wrapping_shr(count as u32) as u8;
            let result = shifted;
            let cf = (value >> (count - 1)) & 1 != 0;
            let of = false; // OF cleared for SAR
            new_flags = flags::compute_flags_u8(flags, result, cf, of, af);
            result
        }
        _ => unreachable!(),
    };

    (result, new_flags)
}

pub fn shift_rm16_cl(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    // Количество позиций = младшие 5 бит регистра CL (для 16-битных операций)
    let count = machine.registers.cl() & 0x1F;

    // Если количество = 0, флаги НЕ изменяются (особый случай!)
    if count == 0 {
        // Для памяти всё равно нужно прочитать и записать значение (но без изменений)
        if !modrm.is_register_mode() {
            let addr = modrm
                .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
                .unwrap();
            bytes.extend_from_slice(&addr.to_le_bytes());
            let value = machine.read_phys_u16(addr);
            machine.write_phys_u16(addr, value);
        }
        machine.log_instruction(csip, &bytes).ok();
        return;
    }

    // Читаем исходное значение из регистра или памяти
    let (value, is_register) = if modrm.is_register_mode() {
        (machine.read_reg16(modrm.rm_field), true)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u16(addr), false)
    };

    // Выполняем операцию в зависимости от reg_field (код операции)
    let result = match modrm.reg_field {
        0 => rol16(value, count, machine),     // ROL
        1 => ror16(value, count, machine),     // ROR
        2 => rcl16(value, count, machine),     // RCL
        3 => rcr16(value, count, machine),     // RCR
        4 | 6 => shl16(value, count, machine), // SHL/SAL (эквивалентны)
        5 => shr16(value, count, machine),     // SHR
        7 => sar16(value, count, machine),     // SAR
        _ => unreachable!(),
    };

    // Сохраняем результат
    if is_register {
        machine.write_reg16(modrm.rm_field, result);
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.write_phys_u16(addr, result);
    }

    machine.log_instruction(csip, &bytes).ok();
}

// Вспомогательные функции для 16-битных операций
fn rol16(value: u16, count: u8, machine: &mut DosMachine) -> u16 {
    let count = count as usize;
    let result = (value << count) | (value >> (16 - count));

    let cf = (value >> (16 - count)) & 1 != 0;
    let of = count == 1 && ((value ^ result) & 0x8000) != 0; // OF для вращения на 1: изменился ли знак?

    let mut flags = machine.registers.flags();
    flags = (flags & !(flags::CF)) | (cf as u16); // CF
    if count == 1 {
        flags = (flags & !(flags::OF)) | ((of as u16) << 11); // OF только для count=1
    }
    machine.registers.set_flags(flags);

    result
}

fn ror16(value: u16, count: u8, machine: &mut DosMachine) -> u16 {
    let count = count as usize;
    let result = (value >> count) | (value << (16 - count));

    // Устанавливаем флаги
    let cf = (value >> (count - 1)) & 1 != 0;
    let of = count == 1 && ((result ^ (result >> 1)) & 0x8000) != 0; // OF для вращения на 1

    let mut flags = machine.registers.flags();
    flags = (flags & !(flags::CF)) | (cf as u16); // CF
    if count == 1 {
        flags = (flags & !(flags::OF)) | ((of as u16) << 11); // OF только для count=1
    }
    machine.registers.set_flags(flags);

    result
}

fn rcl16(value: u16, count: u8, machine: &mut DosMachine) -> u16 {
    let count = count as usize;
    let cf_initial = (machine.registers.flags() & 1) != 0;
    let mut result = value as u32;
    let mut cf = cf_initial;

    // Выполняем вращение на каждую позицию (для корректной установки флагов)
    for _ in 0..count {
        let new_cf = (result & 0x8000) != 0;
        result = (result << 1) | (cf as u32);
        cf = new_cf;
    }

    // Устанавливаем флаги
    let of = count == 1 && (((value as i16) < 0) != ((result as i16) < 0)); // изменился ли знак?

    let mut flags = machine.registers.flags();
    flags = (flags & !(flags::CF)) | (cf as u16); // CF
    if count == 1 {
        flags = (flags & !(flags::OF)) | ((of as u16) << 11); // OF только для count=1
    }
    machine.registers.set_flags(flags);

    result as u16
}

fn rcr16(value: u16, count: u8, machine: &mut DosMachine) -> u16 {
    let count = count as usize;
    let cf_initial = (machine.registers.flags() & 1) != 0;
    let mut result = value as u32;
    let mut cf = cf_initial;

    // Выполняем вращение на каждую позицию
    for _ in 0..count {
        let new_cf = (result & 1) != 0;
        result = (result >> 1) | ((cf as u32) << 15);
        cf = new_cf;
    }

    // Устанавливаем флаги
    let of = count == 1 && (((value as i16) < 0) != ((result as i16) < 0));

    let mut flags = machine.registers.flags();
    flags = (flags & !(flags::CF)) | (cf as u16); // CF
    if count == 1 {
        flags = (flags & !(flags::OF)) | ((of as u16) << 11); // OF только для count=1
    }
    machine.registers.set_flags(flags);

    result as u16
}

fn shl16(value: u16, count: u8, machine: &mut DosMachine) -> u16 {
    let count = count as usize;
    let result = value << count;

    // Флаг переноса = последний вышедший бит
    let cf = if count > 0 && count <= 16 {
        (value >> (16 - count)) & 1 != 0
    } else {
        false
    };

    // Флаг переполнения для сдвига на 1: изменился ли знак?
    let of = count == 1 && ((value ^ result) & 0x8000) != 0;

    // Устанавливаем флаги как при логической операции
    machine
        .registers
        .set_flags(flags::compute_flags_u16(machine.registers.flags(), result, cf, of, false));

    result
}

fn shr16(value: u16, count: u8, machine: &mut DosMachine) -> u16 {
    let count = count as usize;
    let result = value >> count;

    // Флаг переноса = последний вышедший бит
    let cf = (value >> (count - 1)) & 1 != 0;

    // Флаг переполнения для сдвига на 1: всегда = старший бит исходного значения
    let of = count == 1 && (value & 0x8000) != 0;

    // Устанавливаем флаги как при логической операции
    machine
        .registers
        .set_flags(flags::compute_flags_u16(machine.registers.flags(), result, cf, of, false));

    result
}

fn sar16(value: u16, count: u8, machine: &mut DosMachine) -> u16 {
    let count = count as usize;
    let result = ((value as i16) >> count) as u16;

    // Флаг переноса = последний вышедший бит
    let cf = (value >> (count - 1)) & 1 != 0;

    // Флаг переполнения для сдвига на 1: всегда 0 для SAR
    let of = count == 1 && false;

    // Устанавливаем флаги (знак сохраняется)
    machine
        .registers
        .set_flags(flags::compute_flags_u16(machine.registers.flags(), result, cf, of, false));

    result
}

pub fn shift_rm8_cl(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Количество позиций = младшие 5 бит регистра CL, эффективное = count % 8
    let raw_count = machine.registers.cl() & 0x1F;
    let count = if raw_count == 0 { 0 } else { raw_count % 8 };
    
    // Особый случай: если эффективное количество = 0, флаги НЕ изменяются!
    if count == 0 {
        // Для памяти всё равно нужно прочитать и записать значение (но без изменений)
        if !modrm.is_register_mode() {
            let addr = modrm
                .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
                .unwrap();
            bytes.extend_from_slice(&addr.to_le_bytes());
            let value = machine.read_phys_u8(addr);
            machine.write_phys_u8(addr, value);
        }
        machine.log_instruction(csip, &bytes).ok();
        return;
    }
    
    // Читаем исходное значение из регистра или памяти
    let (value, is_register, addr) = if modrm.is_register_mode() {
        (machine.read_reg8(modrm.rm_field), true, 0)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u8(addr), false, addr)
    };
    
    // Выполняем операцию в зависимости от reg_field (код операции)
    let (result, cf, of) = match modrm.reg_field {
        0 => rol8(value, count),        // ROL
        1 => ror8(value, count),        // ROR
        2 => rcl8(value, count, machine.registers.flags() & 1 != 0), // RCL
        3 => rcr8(value, count, machine.registers.flags() & 1 != 0), // RCR
        4 | 6 => shl8(value, count),    // SHL/SAL (эквивалентны)
        5 => shr8(value, count),        // SHR
        7 => sar8(value, count),        // SAR
        _ => unreachable!(),
    };
    
    // Устанавливаем флаги (OF только для сдвига на 1 позицию)
    let mut new_flags = flags::compute_flags_u8(machine.registers.flags(), result, cf, count == 1 && of, false);
    // Сохраняем неизменяемые флаги (если нужно)
    //new_flags = (new_flags & 0x0FD5) | (machine.registers.flags() & !0x0FD5);
    machine.registers.set_flags(new_flags);
    
    // Сохраняем результат
    if is_register {
        machine.write_reg8(modrm.rm_field, result);
    } else {
        machine.write_phys_u8(addr, result);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

// Вспомогательные функции для 8-битных операций
fn rol8(value: u8, count: u8) -> (u8, bool, bool) {
    let count = count as usize;
    let result = (value << count) | (value >> (8 - count));
    let cf = (value >> (8 - count)) & 1 != 0;
    let of = ((value ^ result) & 0x80) != 0; // изменился ли знак (бит 7)?
    (result, cf, of)
}

fn ror8(value: u8, count: u8) -> (u8, bool, bool) {
    let count = count as usize;
    let result = (value >> count) | (value << (8 - count));
    let cf = (value >> (count - 1)) & 1 != 0;
    let of = ((result ^ (result >> 1)) & 0x80) != 0; // SF XOR новый бит 6
    (result, cf, of)
}

fn rcl8(value: u8, count: u8, cf_initial: bool) -> (u8, bool, bool) {
    let count = count as usize % 9; // 9 потому что включаем CF в цикл (8 бит + CF)
    if count == 0 {
        return (value, cf_initial, false);
    }
    
    // Расширяем до 9 бит: [биты 7..0] + [CF]
    let extended = (value as u16) | ((cf_initial as u16) << 8);
    let rotated = (extended << count) | (extended >> (9 - count));
    let result = (rotated & 0xFF) as u8;
    let cf = (rotated & (1 << 8)) != 0;
    let of = ((value as i8) < 0) != ((result as i8) < 0); // изменился ли знак?
    (result, cf, of)
}

fn rcr8(value: u8, count: u8, cf_initial: bool) -> (u8, bool, bool) {
    let count = count as usize % 9;
    if count == 0 {
        return (value, cf_initial, false);
    }
    
    // Расширяем до 9 бит: [CF] + [биты 7..0]
    let extended = ((value as u16) << 1) | (cf_initial as u16);
    let rotated = (extended >> count) | (extended << (9 - count));
    let result = (rotated & 0xFF) as u8;
    let cf = (rotated & 1) != 0;
    let of = ((value as i8) < 0) != ((result as i8) < 0);
    (result, cf, of)
}

fn shl8(value: u8, count: u8) -> (u8, bool, bool) {
    let count = count as usize;
    let result = value << count;
    let cf = if count > 0 { (value >> (8 - count)) & 1 != 0 } else { false }; // последний вышедший бит
    let of = ((value ^ result) & 0x80) != 0; // изменился ли знак?
    (result, cf, of)
}

fn shr8(value: u8, count: u8) -> (u8, bool, bool) {
    let count = count as usize;
    let result = value >> count;
    let cf = if count > 0 { (value >> (count - 1)) & 1 != 0 } else { false }; // последний вышедший бит
    let of = (value & 0x80) != 0; // OF = старший бит исходного значения (для сдвига на 1)
    (result, cf, of)
}

fn sar8(value: u8, count: u8) -> (u8, bool, bool) {
    let count = count as usize;
    let result = ((value as i8) >> count) as u8;
    let cf = if count > 0 { (value >> (count - 1)) & 1 != 0 } else { false }; // последний вышедший бит
    let of = false; // OF всегда 0 для SAR
    (result, cf, of)
}

pub fn shift_group_d0_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None); // продвигаем на байт ModR/M
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let count = 1u8;

    if modrm.is_register_mode() {
        let value = machine.read_reg8(modrm.rm_field);
        let (result, new_flags) = perform_shift_8(modrm.reg_field, value, count, machine.registers.flags());
        machine.write_reg8(modrm.rm_field, result);
        machine.registers.set_flags(new_flags);
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let value = machine.read_phys_u8(addr);
        let (result, new_flags) = perform_shift_8(modrm.reg_field, value, count, machine.registers.flags());
        machine.write_phys_u8(addr, result);
        machine.registers.set_flags(new_flags);
    }

    machine.log_instruction(csip, &bytes).ok();
}