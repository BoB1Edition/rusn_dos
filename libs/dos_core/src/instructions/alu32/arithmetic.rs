// Ver: 2 File: ./libs/dos_core/src/instructions/alu32/arithmetic.rs

use crate::{DosMachine, flags, modrm::ModRm};

pub fn add_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        // ADD reg32, reg32
        let src = machine.read_reg32(modrm.reg_field);
        let dst = machine.read_reg32(modrm.rm_field);
        let res = (dst as u64) + (src as u64);
        let result = res as u32;
        let cf = res > 0xFFFFFFFF;
        let af = ((dst & 0x0F) + (src & 0x0F)) > 0x0F;
        let of = (((dst ^ src) & 0x8000_0000) == 0) && ((dst ^ result) & 0x8000_0000) != 0;
        machine.write_reg32(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_flags_u32(
            machine.registers.flags(),
            result,
            cf,
            of,
            af,
        ));
    } else if modrm.mod_field == 0b00 && modrm.rm_field == 0b110 {
        // [disp16] — разрешено ТОЛЬКО если НЕТ 0x67
        if machine.has_address_size_prefix {
            log::error!("Invalid memory mode for ADD with address-size prefix");
            machine.halted = true;
            return;
        }
        let disp16 = machine.read_instr_u16(machine.registers.ip());
        machine.registers.step(Some(2));
        bytes.extend_from_slice(&disp16.to_le_bytes());
        let phys_addr = (machine.registers.ds() as u32) * 16 + (disp16 as u32);
        let dst_val = machine.read_phys_u32(phys_addr);
        let src_val = machine.read_reg32(modrm.reg_field);
        let res = (dst_val as u64) + (src_val as u64);
        let result = res as u32;
        let cf = res > 0xFFFFFFFF;
        let af = ((dst_val & 0x0F) + (src_val & 0x0F)) > 0x0F;
        let of =
            (((dst_val ^ src_val) & 0x8000_0000) == 0) && ((dst_val ^ result) & 0x8000_0000) != 0;
        machine.write_phys_u32(phys_addr, result);
        machine.registers.set_flags(flags::compute_flags_u32(
            machine.registers.flags(),
            result,
            cf,
            of,
            af,
        ));
    } else {
        let phys_addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&phys_addr.to_le_bytes());
        
        let dst_val = machine.read_phys_u32(phys_addr);
        let src_val = machine.read_reg32(modrm.reg_field);
        let res = (dst_val as u64) + (src_val as u64);
        let result = res as u32;
        let cf = res > 0xFFFFFFFF;
        let af = ((dst_val & 0x0F) + (src_val & 0x0F)) > 0x0F;
        let of = (((dst_val ^ src_val) & 0x8000_0000) == 0) && ((dst_val ^ result) & 0x8000_0000) != 0;
        
        machine.write_phys_u32(phys_addr, result);
        machine.registers.set_flags(flags::compute_flags_u32(
            machine.registers.flags(),
            result,
            cf,
            of,
            af,
        ));
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn sub_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
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
        bytes.extend_from_slice(&phys_addr.to_le_bytes());
        machine.read_phys_u32(phys_addr)
    };
    let dst_reg = modrm.reg_field;
    let dst_val = machine.read_reg32(dst_reg);
    let result = dst_val.wrapping_sub(src_val);
    let cf = dst_val < src_val; // Беззнаковый заём
    let af = (dst_val & 0x0F) < (src_val & 0x0F);
    let of = ((dst_val ^ src_val) & (dst_val ^ result)) & 0x8000_0000 != 0;

    machine.registers.set_flags(flags::compute_flags_u32(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));

    machine.write_reg32(dst_reg, result);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn add_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
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
        machine.read_reg32(modrm.rm_field) // источник: r/m32
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u32(addr)
    };

    let dst_reg = modrm.reg_field; // приёмник: r32
    let dst_val = machine.read_reg32(dst_reg);

    let res = (dst_val as u64) + (src_val as u64);
    let result = res as u32;
    let cf = res > 0xFFFFFFFF;
    let af = ((dst_val & 0x0F) + (src_val & 0x0F)) > 0x0F;
    let of = (((dst_val ^ src_val) & 0x8000_0000) == 0) && ((dst_val ^ result) & 0x8000_0000) != 0;

    machine.write_reg32(dst_reg, result);
    machine.registers.set_flags(flags::compute_flags_u32(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn add_eax_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let imm32 = machine.read_instr_u32(machine.registers.ip());
    machine.registers.step(Some(4));
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm32.to_le_bytes());

    let eax = machine.registers.eax();
    let res = (eax as u64) + (imm32 as u64);
    let result = res as u32;

    // Установка флагов
    let cf = res > 0xFFFFFFFF;
    let zf = result == 0;
    let sf = (result & 0x8000_0000) != 0;
    let af = ((eax & 0x0F) + (imm32 & 0x0F)) > 0x0F;
    let pf = (result as u8).count_ones() % 2 == 0;
    let of = (((eax ^ imm32) & 0x8000_0000) == 0) && ((eax ^ result) & 0x8000_0000) != 0;

    machine.registers.set_flags(flags::compute_flags_u32(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));

    machine.registers.set_eax(result);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn cmp_eax_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let imm32 = machine.read_instr_u32(machine.registers.ip());
    machine.registers.step(Some(4)); // продвигаем на 4 байта (imm32)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm32.to_le_bytes());

    let eax = machine.registers.eax();

    // Вычисление флагов как при вычитании
    let result = eax.wrapping_sub(imm32);
    let cf = eax < imm32;
    let eax_sign = (eax as i32) < 0;
    let imm_sign = (imm32 as i32) < 0;
    let result_sign = (result as i32) < 0;
    let of = (eax_sign != imm_sign) && (eax_sign != result_sign);

    let af = (eax & 0x0F) < (imm32 & 0x0F);

    // Установка флагов
    machine.registers.set_flags(flags::compute_flags_u32(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));

    machine.log_instruction(csip, &bytes).ok();
}

pub fn test_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    // Источник: регистр из reg_field (32-битный)
    let src_val = machine.read_reg32(modrm.reg_field);

    // Приёмник: r/m32 (регистр или память) — читаем, но НЕ записываем результат
    let dst_val = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u32(addr)
    };

    // Вычисляем логическое И (результат НЕ сохраняем!)
    let result = dst_val & src_val;

    // Устанавливаем флаги (логическая операция: CF=0, OF=0)
    machine
        .registers
        .set_flags(flags::compute_logical_flags_u32(
            machine.registers.flags(),
            result,
        ));

    machine.log_instruction(csip, &bytes).ok();
}

pub fn cmp_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
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
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u32(addr)
    };

    let dst_val = machine.read_reg32(modrm.reg_field);
    let res = (dst_val as i64) - (src_val as i64);
    let result = res as u32;
    let cf = dst_val < src_val;
    let af = (dst_val & 0x0F) < (src_val & 0x0F);
    let of =
        (((dst_val ^ src_val) & 0x8000_0000) != 0) && (((dst_val ^ result) & 0x8000_0000) != 0);

    machine.registers.set_flags(flags::compute_flags_u32(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));
    machine.log_instruction(csip, &bytes).ok();
}

pub fn cdq(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let bytes = prev.to_vec();

    // Расширяем знак из EAX в EDX: копируем бит 31 (знак) во все биты EDX
    let eax = machine.registers.eax() as i32;
    let edx = if eax < 0 { 0xFFFFFFFF } else { 0x00000000 };

    machine.registers.set_edx(edx);

    // Флаги НЕ изменяются
    machine.log_instruction(csip, &bytes).ok();
}

/// IMUL r32, r/m32, imm32 — Signed multiply with immediate (32-bit)
/// Регистр ← (источник как знаковое) * (константа как знаковое)
/// Флаги CF/OF = 1 если результат не помещается в 32 бита
pub fn imul_r32_rm32_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    // Читаем источник (регистр или память)
    let src_val = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u32(addr)
    };

    // Читаем 32-битную непосредственную константу (little-endian)
    let imm32 = machine.read_instr_u32(machine.registers.ip());
    machine.registers.step(Some(4));
    bytes.extend_from_slice(&imm32.to_le_bytes());

    // Выполняем знаковое умножение (64-битный промежуточный результат)
    let src_i32 = src_val as i32;
    let imm_i32 = imm32 as i32;
    let result_i64 = src_i32 as i64 * imm_i32 as i64;

    // Усекаем до 32 бит для сохранения в регистр
    let result_u32 = result_i64 as u32;

    // Проверяем переполнение: результат не помещается в 32 бита?
    let result_sign_extended = (result_i64 << 32 >> 32) as i64;
    let overflow = result_i64 != result_sign_extended;

    // Устанавливаем флаги CF и OF
    let mut flags = machine.registers.flags();
    flags = (flags & !(flags::CF)) | (overflow as u16);
    flags = (flags & !(flags::OF)) | ((overflow as u16) << 11);
    machine.registers.set_flags(flags);

    // Сохраняем усечённый результат в регистр назначения
    machine.write_reg32(modrm.reg_field, result_u32);

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn sbb_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
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
    let cf_in = (machine.registers.flags() & flags::CF) != 0;
    let borrow = if cf_in { 1u64 } else { 0u64 };

    if modrm.is_register_mode() {
        let dst_val = machine.read_reg32(modrm.rm_field) as u64;
        let src_extended = src_val as u64 + borrow;
        let (result_u64, did_overflow) = dst_val.overflowing_sub(src_extended);
        let result = result_u64 as u32;
        let new_cf = did_overflow;
        let dst_sign = (dst_val as i32) < 0;
        let src_sign = ((src_extended & 0xFFFF_FFFF) as i32) < 0;
        let result_sign = (result as i32) < 0;
        let new_of = (dst_sign != src_sign) && (dst_sign != result_sign);
        let dst_low = dst_val & 0x0F;
        let src_low = src_extended & 0x0F;
        let new_af = dst_low < src_low;

        machine.write_reg32(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_flags_u32(
            machine.registers.flags(),
            result,
            new_cf,
            new_of,
            new_af,
        ));
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let dst_val = machine.read_phys_u32(addr) as u64;
        let src_extended = src_val as u64 + borrow;
        let (result_u64, did_overflow) = dst_val.overflowing_sub(src_extended);
        let result = result_u64 as u32;
        let new_cf = did_overflow;
        let dst_sign = (dst_val as i32) < 0;
        let src_sign = ((src_extended & 0xFFFF_FFFF) as i32) < 0;
        let result_sign = (result as i32) < 0;
        let new_of = (dst_sign != src_sign) && (dst_sign != result_sign);
        let dst_low = dst_val & 0x0F;
        let src_low = src_extended & 0x0F;
        let new_af = dst_low < src_low;

        machine.write_phys_u32(addr, result);
        machine.registers.set_flags(flags::compute_flags_u32(
            machine.registers.flags(),
            result,
            new_cf,
            new_of,
            new_af,
        ));
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub fn adc_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = machine.read_reg32(modrm.reg_field);
    let cf_in = (machine.registers.flags() & flags::CF) != 0;
    let (dst_val, is_mem, phys_addr) = if modrm.is_register_mode() {
        (machine.read_reg32(modrm.rm_field), false, 0)
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        (machine.read_phys_u32(addr), true, addr)
    };

    let sum = (dst_val as u64) + (src_val as u64) + (if cf_in { 1 } else { 0 });
    let result = sum as u32;
    let new_cf = sum > 0xFFFF_FFFF;
    let new_af = ((dst_val & 0x0F) + (src_val & 0x0F) + (if cf_in { 1 } else { 0 })) > 0x0F;
    let new_of = ((dst_val ^ src_val) & 0x8000_0000) == 0 && ((dst_val ^ result) & 0x8000_0000) != 0;

    machine.registers.set_flags(flags::compute_flags_u32(machine.registers.flags(), result, new_cf, new_of, new_af));

    if is_mem {
        machine.write_phys_u32(phys_addr, result);
    } else {
        machine.write_reg32(modrm.rm_field, result);
    }
    machine.log_instruction(csip, &bytes).ok();
}

/// ADC r32, r/m32
pub fn adc_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u32(addr)
    };

    let dst_reg = modrm.reg_field;
    let dst_val = machine.read_reg32(dst_reg);
    let cf_in = (machine.registers.flags() & flags::CF) != 0;
    let sum = (dst_val as u64) + (src_val as u64) + (if cf_in { 1 } else { 0 });
    let result = sum as u32;
    let new_cf = sum > 0xFFFF_FFFF;
    let new_af = ((dst_val & 0x0F) as u64 + (src_val & 0x0F) as u64 + (if cf_in { 1 } else { 0 })) > 0x0F;
    let new_of = ((dst_val ^ src_val) & 0x8000_0000) == 0 && ((dst_val ^ result) & 0x8000_0000) != 0;

    machine.write_reg32(dst_reg, result);
    machine.registers.set_flags(flags::compute_flags_u32(machine.registers.flags(), result, new_cf, new_of, new_af));
    machine.log_instruction(csip, &bytes).ok();
}

pub fn sbb_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u32(addr)
    };

    let dst_reg = modrm.reg_field;
    let dst_val = machine.read_reg32(dst_reg);
    let cf_in = (machine.registers.flags() & flags::CF) != 0;
    let borrow = if cf_in { 1u64 } else { 0u64 };

    let result = (dst_val as u64).wrapping_sub(src_val as u64 + borrow) as u32;
    let new_cf = (dst_val as u64) < (src_val as u64 + borrow);
    let new_af = (dst_val & 0x0F) < ((src_val & 0x0F) + borrow as u32);
    let new_of = ((dst_val ^ src_val) & 0x8000_0000) != 0 && ((dst_val ^ result) & 0x8000_0000) != 0;

    machine.write_reg32(dst_reg, result);
    machine.registers.set_flags(flags::compute_flags_u32(machine.registers.flags(), result, new_cf, new_of, new_af));
    machine.log_instruction(csip, &bytes).ok();
}

/// SUB r/m32, r32 (опкод 0x29 с префиксом 0x66)
pub(crate) fn sub_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
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
        let dst_val = machine.read_reg32(modrm.rm_field);
        let result = dst_val.wrapping_sub(src_val);
        let cf = dst_val < src_val;
        let af = (dst_val & 0x0F) < (src_val & 0x0F);
        let of = ((dst_val ^ src_val) & 0x8000_0000) != 0 && ((dst_val ^ result) & 0x8000_0000) != 0;
        machine.write_reg32(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_flags_u32(
            machine.registers.flags(),
            result,
            cf,
            of,
            af,
        ));
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let dst_val = machine.read_phys_u32(addr);
        let result = dst_val.wrapping_sub(src_val);
        let cf = dst_val < src_val;
        let af = (dst_val & 0x0F) < (src_val & 0x0F);
        let of = ((dst_val ^ src_val) & 0x8000_0000) != 0 && ((dst_val ^ result) & 0x8000_0000) != 0;
        machine.write_phys_u32(addr, result);
        machine.registers.set_flags(flags::compute_flags_u32(
            machine.registers.flags(),
            result,
            cf,
            of,
            af,
        ));
    }

    machine.log_instruction(csip, &bytes).ok();
}

/// CMP r/m32, r32 (опкод 0x39 с префиксом 0x66)
pub(crate) fn cmp_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
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
    let dst_val = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u32(addr)
    };

    let result = dst_val.wrapping_sub(src_val);
    let cf = dst_val < src_val;
    let af = (dst_val & 0x0F) < (src_val & 0x0F);
    let of = ((dst_val ^ src_val) & 0x8000_0000) != 0 && ((dst_val ^ result) & 0x8000_0000) != 0;

    machine.registers.set_flags(flags::compute_flags_u32(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));

    machine.log_instruction(csip, &bytes).ok();
}