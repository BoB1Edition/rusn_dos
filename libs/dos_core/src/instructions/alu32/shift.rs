// Ver: 1
use crate::{DosMachine, flags, modrm::ModRm};

fn perform_shift(op_field: u8, value: u32, count: u8, flags: u16) -> (u32, u16) {
    let count = count & 0x1F; // x86: count mod 32
    if count == 0 {
        return (value, flags);
    }

    let mut new_flags = flags;
    let af = false;
    let result = match op_field {
        0 => {
            // ROL
            let result = value.rotate_left(count as u32);
            let cf = (result >> 31) != 0;
            let msb_before = (value >> 31) != 0;
            let msb_after = (result >> 31) != 0;
            let of = if count == 1 {
                msb_before != msb_after
            } else {
                false
            };
            new_flags = flags::compute_flags_u32(result, cf, of, af);
            result
        }
        1 => {
            // ROR
            let result = value.rotate_right(count as u32);
            let cf = (result & 1) != 0;
            let lsb_before = (value & 1) != 0;
            let msb_after = (result >> 31) != 0;
            let of = if count == 1 {
                lsb_before != msb_after
            } else {
                false
            };
            new_flags = flags::compute_flags_u32(result, cf, of, af);
            result
        }
        2 => {
            // RCL - Rotate Left through Carry
            let carry_in = (flags & 1) != 0;
            let input = (value as u64) | ((carry_in as u64) << 32);
            let rotated = input.rotate_left(count as u32);
            let result = rotated as u32;
            let new_cf = (rotated >> 32) != 0;
            let msb_before = (value >> 31) != 0;
            let msb_after = (result >> 31) != 0;
            let of = if count == 1 {
                msb_before != msb_after
            } else {
                false
            };
            new_flags = flags::compute_flags_u32(result, new_cf, of, af);
            result
        }
        3 => {
            // RCR - Rotate Right through Carry
            let carry_in = (flags & 1) != 0;
            let input = (value as u64) | ((carry_in as u64) << 32);
            let rotated = input.rotate_right(count as u32);
            let result = rotated as u32;
            let new_cf = (rotated >> 32) != 0;
            let of = if count == 1 {
                let msb = (result >> 31) & 1;
                let second_msb = (result >> 30) & 1;
                msb != second_msb
            } else {
                false
            };
            new_flags = flags::compute_flags_u32(result, new_cf, of, af);
            result
        }
        4 | 6 => {
            // SHL / SAL - Shift Left Arithmetic/Logical (одинаково)
            let extended = value as u64;
            let shifted = extended << count;
            let result = shifted as u32;
            let cf = (shifted >> 32) != 0;
            let of = if count == 1 {
                let msb_result = (result >> 31) & 1;
                let cf_bit = cf as u32;
                (msb_result ^ cf_bit) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u32(result, cf, of, af);
            result
        }
        5 => {
            // SHR - Shift Right Logical
            let shifted = value >> count;
            let result = shifted;
            let cf = (value >> (count - 1)) & 1 != 0;
            let of = if count == 1 {
                (((result >> 31) ^ (cf as u32)) & 1) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u32(result, cf, of, af);
            result
        }
        7 => {
            // SAR - Shift Right Arithmetic
            let shifted = (value as i32).wrapping_shr(count as u32) as u32;
            let result = shifted;
            let cf = if count == 0 {
                false
            } else {
                (value >> (count - 1)) & 1 != 0
            };
            let of = false; // OF cleared for SAR
            new_flags = flags::compute_flags_u32(result, cf, of, af);
            result
        }
        _ => unreachable!(),
    };

    (result, new_flags)
}

pub fn shift_group_d1_32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None); // продвигаем только на байт ModR/M
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        // ROL/ROR/RCL/RCR/SHL/SHR/SAR reg32, 1
        let value = machine.read_reg32(modrm.rm_field);
        let (result, new_flags) =
            perform_shift(modrm.reg_field, value, 1, machine.registers.flags());
        machine.write_reg32(modrm.rm_field, result);
        machine.registers.set_flags(new_flags);
    } else {
        // ROL/ROR/RCL/RCR/SHL/SHR/SAR [mem], 1
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let value = machine.read_phys_u32(addr);
        let (result, new_flags) =
            perform_shift(modrm.reg_field, value, 1, machine.registers.flags());
        machine.write_phys_u32(addr, result);
        machine.registers.set_flags(new_flags);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub fn shift_group_c1_32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    let imm8 = machine.read_u8(
        machine.registers.cs(),
        machine.registers.ip().wrapping_add(1),
    );
    machine.registers.step(Some(2));
    bytes.push(modrm_byte);
    bytes.push(imm8);
    let modrm = ModRm::from_byte(modrm_byte);

    let (value, addr_opt) = if modrm.is_register_mode() {
        (machine.read_reg32(modrm.rm_field), None)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u32(addr), Some(addr))
    };

    let (result, new_flags) =
        perform_shift(modrm.reg_field, value, imm8, machine.registers.flags());

    if let Some(addr) = addr_opt {
        // Запись обратно в память
        machine.write_phys_u32(addr, result);
    } else {
        // Запись в регистр
        machine.write_reg32(modrm.rm_field, result);
    }

    machine.registers.set_flags(new_flags);
    machine.log_instruction(csip, &bytes).ok();
}

/// Обработка опкода 0xD3 с префиксом 0x66 — 32-битные сдвиги/вращения
pub fn shift_rm32_cl(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    // Количество позиций = младшие 5 бит регистра CL (для 32-битных операций: 0..31)
    let count = machine.registers.cl() & 0x1F;

    // Особый случай: если количество = 0, флаги НЕ изменяются!
    if count == 0 {
        if !modrm.is_register_mode() {
            let addr = modrm
                .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
                .unwrap();
            bytes.extend_from_slice(&addr.to_le_bytes());
            let value = machine.read_phys_u32(addr);
            machine.write_phys_u32(addr, value);
        }
        machine.log_instruction(csip, &bytes).ok();
        return;
    }

    // Читаем исходное значение из регистра или памяти
    let (value, is_register, addr) = if modrm.is_register_mode() {
        (machine.read_reg32(modrm.rm_field), true, 0)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u32(addr), false, addr)
    };

    // Выполняем операцию
    let (result, cf, of) = match modrm.reg_field {
        0 => rol32(value, count),
        1 => ror32(value, count),
        2 => rcl32(value, count, machine.registers.flags() & 1 != 0),
        3 => rcr32(value, count, machine.registers.flags() & 1 != 0),
        4 | 6 => shl32(value, count),
        5 => shr32(value, count),
        7 => sar32(value, count),
        _ => unreachable!(),
    };

    // Устанавливаем флаги (OF только для сдвига на 1 позицию)
    let mut new_flags = flags::compute_flags_u32(result, cf, count == 1 && of, false);
    // Сохраняем неизменяемые флаги
    new_flags = (new_flags & 0x0FD5) | (machine.registers.flags() & !0x0FD5);
    machine.registers.set_flags(new_flags as u16);

    // Сохраняем результат
    if is_register {
        machine.write_reg32(modrm.rm_field, result);
    } else {
        machine.write_phys_u32(addr, result);
    }

    machine.log_instruction(csip, &bytes).ok();
}

// Вспомогательные функции для 32-битных операций
fn rol32(value: u32, count: u8) -> (u32, bool, bool) {
    let count = count as usize % 32;
    let result = (value << count) | (value >> (32 - count));
    let cf = (result & 1) != 0;
    let of = ((value ^ result) & 0x80000000) != 0;
    (result, cf, of)
}

fn ror32(value: u32, count: u8) -> (u32, bool, bool) {
    let count = count as usize % 32;
    let result = (value >> count) | (value << (32 - count));
    let cf = (result & 0x80000000) != 0;
    let of = ((result ^ (result >> 1)) & 0x80000000) != 0;
    (result, cf, of)
}

fn rcl32(value: u32, count: u8, cf_initial: bool) -> (u32, bool, bool) {
    let count = count as usize % 33; // 33 потому что включаем CF в цикл
    if count == 0 {
        return (value, cf_initial, false);
    }

    let extended = (value as u64) | ((cf_initial as u64) << 32);
    let rotated = (extended << count) | (extended >> (33 - count));
    let result = (rotated & 0xFFFFFFFF) as u32;
    let cf = (rotated & (1 << 32)) != 0;
    let of = ((value as i32) < 0) != ((result as i32) < 0);
    (result, cf, of)
}

fn rcr32(value: u32, count: u8, cf_initial: bool) -> (u32, bool, bool) {
    let count = count as usize % 33;
    if count == 0 {
        return (value, cf_initial, false);
    }

    let extended = ((value as u64) << 1) | (cf_initial as u64);
    let rotated = (extended >> count) | (extended << (33 - count));
    let result = (rotated & 0xFFFFFFFF) as u32;
    let cf = (rotated & 1) != 0;
    let of = ((value as i32) < 0) != ((result as i32) < 0);
    (result, cf, of)
}

fn shl32(value: u32, count: u8) -> (u32, bool, bool) {
    let count = count as usize % 32;
    let result = value << count;
    let cf = if count > 0 {
        (value >> (32 - count)) & 1 != 0
    } else {
        false
    };
    let of = ((value ^ result) & 0x80000000) != 0;
    (result, cf, of)
}

fn shr32(value: u32, count: u8) -> (u32, bool, bool) {
    let count = count as usize % 32;
    let result = value >> count;
    let cf = if count > 0 {
        (value >> (count - 1)) & 1 != 0
    } else {
        false
    };
    let of = (value & 0x80000000) != 0;
    (result, cf, of)
}

fn sar32(value: u32, count: u8) -> (u32, bool, bool) {
    let count = count as usize % 32;
    let result = ((value as i32) >> count) as u32;
    let cf = if count > 0 {
        (value >> (count - 1)) & 1 != 0
    } else {
        false
    };
    let of = false;
    (result, cf, of)
}
