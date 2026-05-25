// Ver: 1 File: ./libs/dos_core/src/instructions/alu/arithmetic.rs
use crate::{DosMachine, flags, modrm::ModRm};

pub(crate) fn add_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = machine.read_reg8(modrm.reg_field);

    let (dst_val, is_register, addr) = if modrm.is_register_mode() {
        (machine.read_reg8(modrm.rm_field), true, 0)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u8(addr), false, addr)
    };

    let res = (dst_val as u16) + (src_val as u16);
    let result = res as u8;

    let cf = res > 0xFF;
    let af = ((dst_val & 0x0F) + (src_val & 0x0F)) > 0x0F;
    let of = (((dst_val ^ src_val) & 0x80) == 0) && ((dst_val ^ result) & 0x80) != 0;
    if is_register {
        machine.write_reg8(modrm.rm_field, result);
    } else {
        machine.write_phys_u8(addr, result);
    }

    machine.registers.set_flags(flags::compute_flags_u8(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn add_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
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
        machine.read_reg16(modrm.reg_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u16(addr)
    };
    do_add_16(machine, modrm.rm_field, src_val);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn add_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
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
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u16(addr)
    };
    do_add_16(machine, modrm.reg_field, src_val);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn add_al_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let imm8 = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(imm8);
    let al = machine.registers.al();
    let result = (al as u16) + (imm8 as u16);
    let result8 = result as u8;
    let cf = result > 0xFF;
    let af = ((al & 0x0F) + (imm8 & 0x0F)) > 0x0F;
    let al_sign = (al as i8) < 0;
    let imm_sign = (imm8 as i8) < 0;
    let result_sign = (result8 as i8) < 0;
    let of = (al_sign == imm_sign) && (result_sign != al_sign);
    machine.registers.set_flags(flags::compute_flags_u8(
        machine.registers.flags(),
        result8,
        cf,
        of,
        af,
    ));
    machine.registers.set_al(result8);
    machine.log_instruction(csip, &bytes).ok();
}

// Внутренняя функция — не вызывается напрямую из execute()
fn do_add_16(machine: &mut DosMachine, dst_reg: u8, src_value: u16) {
    let dst_val = machine.read_reg16(dst_reg);
    let res = dst_val as u32 + src_value as u32;
    let result = res as u16;
    let cf = res > 0xFFFF;
    let af = ((dst_val & 0x0F) + (src_value & 0x0F)) > 0x0F;
    let of = (((dst_val ^ src_value) & 0x8000) == 0) && ((dst_val ^ result) & 0x8000) != 0;

    machine.write_reg16(dst_reg, result);
    machine.registers.set_flags(flags::compute_flags_u16(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));
}

pub fn add_ax_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let imm16 = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2));
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());

    let ax = machine.registers.ax();
    let res = (ax as u32) + (imm16 as u32);
    let result = res as u16;

    // Установка флагов
    let cf = res > 0xFFFF;
    let af = ((ax & 0x0F) + (imm16 & 0x0F)) > 0x0F;
    let of = (((ax ^ imm16) & 0x8000) == 0) && ((ax ^ result) & 0x8000) != 0;

    machine.registers.set_flags(flags::compute_flags_u16(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));

    machine.registers.set_ax(result);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn cmp_ax_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let imm16 = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2)); // продвигаем на 2 байта (imm16)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());

    let ax = machine.registers.ax();
    let result = ax.wrapping_sub(imm16);

    let cf = ax < imm16;

    let ax_sign = (ax as i16) < 0;
    let imm_sign = (imm16 as i16) < 0;
    let result_sign = (result as i16) < 0;
    let of = (ax_sign != imm_sign) && (ax_sign != result_sign);

    let af = (ax & 0x0F) < (imm16 & 0x0F);
    machine.registers.set_flags(flags::compute_flags_u16(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn sbb_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let src_val = machine.read_reg8(modrm.reg_field);
    let cf = (machine.registers.flags() & 1) != 0;
    let borrow = if cf { 1u16 } else { 0u16 };
    if modrm.is_register_mode() {
        let dst_val = machine.read_reg8(modrm.rm_field) as u16;
        let src_extended = src_val as u16 + borrow;
        let (result_u16, did_overflow) = dst_val.overflowing_sub(src_extended);
        let result = result_u16 as u8;
        let new_cf = did_overflow;
        let dst_sign = (dst_val as i8) < 0;
        let src_sign = ((src_extended & 0xFF) as i8) < 0;
        let result_sign = (result as i8) < 0;
        let new_of = (dst_sign != src_sign) && (dst_sign != result_sign);
        let dst_low = dst_val & 0x0F;
        let src_low = src_extended & 0x0F;
        let new_af = dst_low < src_low;
        machine.write_reg8(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_flags_u8(
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
        let dst_val = machine.read_phys_u8(addr) as u16;
        let src_extended = src_val as u16 + borrow;

        let (result_u16, did_overflow) = dst_val.overflowing_sub(src_extended);
        let result = result_u16 as u8;
        let new_cf = did_overflow;
        let dst_sign = (dst_val as i8) < 0;
        let src_sign = ((src_extended & 0xFF) as i8) < 0;
        let result_sign = (result as i8) < 0;
        let new_of = (dst_sign != src_sign) && (dst_sign != result_sign);
        let dst_low = dst_val & 0x0F;
        let src_low = src_extended & 0x0F;
        let new_af = dst_low < src_low;
        machine.write_phys_u8(addr, result);
        machine.registers.set_flags(flags::compute_flags_u8(
            machine.registers.flags(),
            result,
            new_cf,
            new_of,
            new_af,
        ));
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn sub_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
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
        let src_val = machine.read_reg16(modrm.reg_field);
        let dst_val = machine.read_reg16(modrm.rm_field);
        let result = dst_val.wrapping_sub(src_val);
        let cf = dst_val < src_val;
        let af = (dst_val & 0x0F) < (src_val & 0x0F);
        let of = (((dst_val ^ src_val) & 0x8000) != 0) && (((dst_val ^ result) & 0x8000) != 0);
        machine.write_reg16(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_flags_u16(
            machine.registers.flags(),
            result,
            cf,
            of,
            af,
        ));
    } else {
        // [NEW] Путь для памяти
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let src_val = machine.read_reg16(modrm.reg_field);
        let dst_val = machine.read_phys_u16(addr);
        let result = dst_val.wrapping_sub(src_val);
        let cf = dst_val < src_val;
        let af = (dst_val & 0x0F) < (src_val & 0x0F);
        let of = (((dst_val ^ src_val) & 0x8000) != 0) && (((dst_val ^ result) & 0x8000) != 0);
        machine.write_phys_u16(addr, result);
        machine.registers.set_flags(flags::compute_flags_u16(
            machine.registers.flags(),
            result,
            cf,
            of,
            af,
        ));
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub fn cmp_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
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
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u16(addr)
    };

    let dst_val = machine.read_reg16(modrm.reg_field);
    let res = (dst_val as i32) - (src_val as i32);
    let result = res as u16;
    let cf = (dst_val as u32) < (src_val as u32);
    let af = (dst_val & 0x0F) < (src_val & 0x0F);
    let of = (((dst_val ^ src_val) & 0x8000) != 0) && (((dst_val ^ result) & 0x8000) != 0);

    machine.registers.set_flags(flags::compute_flags_u16(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));
    machine.log_instruction(csip, &bytes).ok();
}

pub fn cmp_al_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let imm8 = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(imm8);

    let al = machine.registers.al();
    let result = al.wrapping_sub(imm8);
    let cf = al < imm8;
    let al_sign = (al as i8) < 0;
    let imm_sign = (imm8 as i8) < 0;
    let result_sign = (result as i8) < 0;
    let of = (al_sign != imm_sign) && (al_sign != result_sign);
    let af = (al & 0x0F) < (imm8 & 0x0F);
    machine.registers.set_flags(flags::compute_flags_u8(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));

    machine.log_instruction(csip, &bytes).ok();
}


pub fn test_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let src_val = machine.read_reg8(modrm.reg_field);
    let dst_val = if modrm.is_register_mode() {
        machine.read_reg8(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u8(addr)
    };
    let result = dst_val & src_val;
    machine.registers.set_flags(flags::compute_logical_flags_u8(
        machine.registers.flags(),
        result,
    ));

    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/alu/arithmetic.rs
pub fn add_r8_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    // Источник: r/m8 (регистр или память)
    let src_val = if modrm.is_register_mode() {
        machine.read_reg8(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u8(addr)
    };

    // Приёмник: регистр из reg_field
    let dst_val = machine.read_reg8(modrm.reg_field);

    // Выполняем сложение с установкой флагов
    let result = dst_val.wrapping_add(src_val);
    let cf = dst_val as u16 + src_val as u16 > 0xFF;
    let dst_sign = (dst_val as i8) < 0;
    let src_sign = (src_val as i8) < 0;
    let result_sign = (result as i8) < 0;
    let of = (dst_sign == src_sign) && (dst_sign != result_sign);
    let af = ((dst_val & 0x0F) + (src_val & 0x0F)) > 0x0F;
    machine.write_reg8(modrm.reg_field, result);
    machine.registers.set_flags(flags::compute_flags_u8(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));

    machine.log_instruction(csip, &bytes).ok();
}

pub fn test_al_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let imm8 = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None); // продвигаем на 1 байт (imm8)
    let mut bytes = prev.to_vec();
    bytes.push(imm8);
    let al = machine.registers.al();
    let result = al & imm8;
    machine.registers.set_flags(flags::compute_logical_flags_u8(
        machine.registers.flags(),
        result,
    ));

    machine.log_instruction(csip, &bytes).ok();
}

pub fn cmp_r8_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    // Источник: r/m8 (регистр или память)
    let src_val = if modrm.is_register_mode() {
        machine.read_reg8(modrm.rm_field)
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u8(addr)
    };
    let dst_val = machine.read_reg8(modrm.reg_field);
    let result = dst_val.wrapping_sub(src_val);
    let cf = dst_val < src_val;
    let dst_sign = (dst_val as i8) < 0;
    let src_sign = (src_val as i8) < 0;
    let result_sign = (result as i8) < 0;
    let of = (dst_sign != src_sign) && (dst_sign != result_sign);
    let af = (dst_val & 0x0F) < (src_val & 0x0F);
    machine.registers.set_flags(flags::compute_flags_u8(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));

    machine.log_instruction(csip, &bytes).ok();
}

pub fn cwd(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    bytes.push(0x99); // опкод CWD
    let ax = machine.registers.ax() as i16;
    let dx = if ax < 0 { 0xFFFF } else { 0x0000 };

    machine.registers.set_dx(dx);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn cbw(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    bytes.push(0x98); // опкод CBW
    let al = machine.registers.al() as i8;
    let ah = if al < 0 { 0xFF } else { 0x00 };

    machine.registers.set_ah(ah);
    machine.log_instruction(csip, &bytes).ok();
}

/// IMUL r16, r/m16, imm16 — Signed multiply with immediate (16-bit)
/// Регистр ← (источник как знаковое) * (константа как знаковое)
/// Флаги CF/OF = 1 если результат не помещается в 16 бит
pub fn imul_r16_rm16_imm16(machine: &mut DosMachine, prev: &[u8]) {
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
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u16(addr)
    };

    let imm16 = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&imm16.to_le_bytes());
    let src_i16 = src_val as i16;
    let imm_i16 = imm16 as i16;
    let result_i32 = src_i16 as i32 * imm_i16 as i32;
    let result_u16 = result_i32 as u16;
    let result_sign_extended = (result_i32 << 16 >> 16) as i32; // знаковое расширение младших 16 бит
    let overflow = result_i32 != result_sign_extended;
    let mut flags = machine.registers.flags();
    flags = (flags & !(flags::CF)) | (overflow as u16); // CF = overflow
    flags = (flags & !(flags::OF)) | ((overflow as u16) << 11); // OF = overflow
    machine.registers.set_flags(flags);

    machine.write_reg16(modrm.reg_field, result_u16);

    machine.log_instruction(csip, &bytes).ok();
}

pub fn adc_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = machine.read_reg8(modrm.reg_field);
    let cf_in = (machine.registers.flags() & flags::CF) != 0;
    let (dst_val, is_mem, phys_addr) = if modrm.is_register_mode() {
        (machine.read_reg8(modrm.rm_field), false, 0)
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        (machine.read_phys_u8(addr), true, addr)
    };

    let sum = (dst_val as u16) + (src_val as u16) + (if cf_in { 1 } else { 0 });
    let result = sum as u8;
    let new_cf = sum > 0xFF;
    let new_af = ((dst_val & 0x0F) as u16 + (src_val & 0x0F) as u16 + (if cf_in { 1 } else { 0 })) > 0x0F;
    let new_of = ((dst_val ^ src_val) & 0x80) == 0 && ((dst_val ^ result) & 0x80) != 0;

    machine.registers.set_flags(flags::compute_flags_u8(machine.registers.flags(), result, new_cf, new_of, new_af));

    // Запись результата
    if is_mem {
        machine.write_phys_u8(phys_addr, result);
    } else {
        machine.write_reg8(modrm.rm_field, result);
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn sub_al_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let imm8 = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(imm8);

    let al = machine.registers.al();
    let result = al.wrapping_sub(imm8);
    let cf = al < imm8;
    let af = (al & 0x0F) < (imm8 & 0x0F);
    let of = ((al ^ imm8) & 0x80) != 0 && ((al ^ result) & 0x80) != 0;
    machine.registers.set_flags(flags::compute_flags_u8(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));
    machine.registers.set_al(result);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn sub_r8_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    // Источник: r/m8
    let src_val = if modrm.is_register_mode() {
        machine.read_reg8(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u8(addr)
    };

    // Приёмник: регистр из reg_field
    let dst_val = machine.read_reg8(modrm.reg_field);
    let result = dst_val.wrapping_sub(src_val);
    let cf = dst_val < src_val;
    let af = (dst_val & 0x0F) < (src_val & 0x0F);
    let of = ((dst_val ^ src_val) & 0x80) != 0 && ((dst_val ^ result) & 0x80) != 0;

    machine.write_reg8(modrm.reg_field, result);
    machine.registers.set_flags(flags::compute_flags_u8(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn sbb_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = machine.read_reg16(modrm.reg_field);
    let cf_in = (machine.registers.flags() & flags::CF) != 0;
    let borrow = if cf_in { 1u32 } else { 0u32 };

    if modrm.is_register_mode() {
        let dst_val = machine.read_reg16(modrm.rm_field) as u32;
        let src_extended = src_val as u32 + borrow;
        let (result_u32, did_overflow) = dst_val.overflowing_sub(src_extended);
        let result = result_u32 as u16;
        let new_cf = did_overflow;
        let dst_sign = (dst_val as i16) < 0;
        let src_sign = ((src_extended & 0xFFFF) as i16) < 0;
        let result_sign = (result as i16) < 0;
        let new_of = (dst_sign != src_sign) && (dst_sign != result_sign);
        let dst_low = dst_val & 0x0F;
        let src_low = src_extended & 0x0F;
        let new_af = dst_low < src_low;

        machine.write_reg16(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_flags_u16(
            machine.registers.flags(), result, new_cf, new_of, new_af));
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let dst_val = machine.read_phys_u16(addr) as u32;
        let src_extended = src_val as u32 + borrow;
        let (result_u32, did_overflow) = dst_val.overflowing_sub(src_extended);
        let result = result_u32 as u16;
        let new_cf = did_overflow;
        let dst_sign = (dst_val as i16) < 0;
        let src_sign = ((src_extended & 0xFFFF) as i16) < 0;
        let result_sign = (result as i16) < 0;
        let new_of = (dst_sign != src_sign) && (dst_sign != result_sign);
        let dst_low = dst_val & 0x0F;
        let src_low = src_extended & 0x0F;
        let new_af = dst_low < src_low;

        machine.write_phys_u16(addr, result);
        machine.registers.set_flags(flags::compute_flags_u16(
            machine.registers.flags(), result, new_cf, new_of, new_af));
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn adc_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = machine.read_reg16(modrm.reg_field);
    let cf_in = (machine.registers.flags() & flags::CF) != 0;
    let (dst_val, is_mem, phys_addr) = if modrm.is_register_mode() {
        (machine.read_reg16(modrm.rm_field), false, 0)
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        (machine.read_phys_u16(addr), true, addr)
    };

    let sum = (dst_val as u32) + (src_val as u32) + (if cf_in { 1 } else { 0 });
    let result = sum as u16;
    let new_cf = sum > 0xFFFF;
    let new_af = ((dst_val & 0x0F) + (src_val & 0x0F) + (if cf_in { 1 } else { 0 })) > 0x0F;
    let new_of = ((dst_val ^ src_val) & 0x8000) == 0 && ((dst_val ^ result) & 0x8000) != 0;

    machine.registers.set_flags(flags::compute_flags_u16(machine.registers.flags(), result, new_cf, new_of, new_af));

    if is_mem {
        machine.write_phys_u16(phys_addr, result);
    } else {
        machine.write_reg16(modrm.rm_field, result);
    }
    machine.log_instruction(csip, &bytes).ok();
}

/// ADC r16, r/m16 — приёмник в регистре reg, источник r/m16, плюс CF
pub(crate) fn adc_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    // Источник: r/m16
    let src_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u16(addr)
    };

    let dst_reg = modrm.reg_field;          // приёмник
    let dst_val = machine.read_reg16(dst_reg);
    let cf_in = (machine.registers.flags() & flags::CF) != 0;
    let sum = (dst_val as u32) + (src_val as u32) + (if cf_in { 1 } else { 0 });
    let result = sum as u16;
    let new_cf = sum > 0xFFFF;
    let new_af = ((dst_val & 0x0F) as u32 + (src_val & 0x0F) as u32 + (if cf_in { 1 } else { 0 })) > 0x0F;
    let new_of = ((dst_val ^ src_val) & 0x8000) == 0 && ((dst_val ^ result) & 0x8000) != 0;

    machine.write_reg16(dst_reg, result);
    machine.registers.set_flags(flags::compute_flags_u16(machine.registers.flags(), result, new_cf, new_of, new_af));
    machine.log_instruction(csip, &bytes).ok();
}