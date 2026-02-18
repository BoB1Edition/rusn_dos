// Ver: 1

use crate::{DosMachine, flags, modrm::ModRm};


pub fn add_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
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
        // Установка флагов...
        let mut flags = machine.registers.flags();
        flags &= !(1 << 0 | 1 << 4 | 1 << 6 | 1 << 7 | 1 << 11);
        if result == 0 {
            flags |= 1 << 6;
        }
        if (result & 0x8000_0000) != 0 {
            flags |= 1 << 7;
        }
        if cf {
            flags |= 1 << 0;
        }
        if of {
            flags |= 1 << 11;
        }
        if af {
            flags |= 1 << 4;
        }
        machine.registers.set_flags(flags);
    } else if modrm.mod_field == 0b00 && modrm.rm_field == 0b110 {
        // [disp16] — разрешено ТОЛЬКО если НЕТ 0x67
        if machine.has_address_size_prefix {
            log::error!("Invalid memory mode for ADD with address-size prefix");
            machine.halted = true;
            return;
        }
        let disp16 = machine.read_u16(machine.registers.cs(), machine.registers.ip());
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
        // Установка флагов...
        let mut flags = machine.registers.flags();
        flags &= !(1 << 0 | 1 << 4 | 1 << 6 | 1 << 7 | 1 << 11);
        if result == 0 {
            flags |= 1 << 6;
        }
        if (result & 0x8000_0000) != 0 {
            flags |= 1 << 7;
        }
        if cf {
            flags |= 1 << 0;
        }
        if of {
            flags |= 1 << 11;
        }
        if af {
            flags |= 1 << 4;
        }
        machine.registers.set_flags(flags);
    } else {
        log::error!("Unsupported memory mode in ADD r/m32, r32");
        machine.halted = true;
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub fn sub_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    machine.log_instruction(csip, &bytes).ok();

    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        // SUB reg32, reg32
        let dst_val = machine.read_reg32(modrm.reg_field);
        let src_val = machine.read_reg32(modrm.rm_field);
        let res = (dst_val as i64) - (src_val as i64);
        let result = res as u32;
        let cf = dst_val < src_val;
        let af = (dst_val & 0x0F) < (src_val & 0x0F);
        let of =
            (((dst_val ^ src_val) & 0x8000_0000) != 0) && (((dst_val ^ result) & 0x8000_0000) != 0);
        machine.write_reg32(modrm.reg_field, result);

        let mut flags = machine.registers.flags();
        flags &= !(1 << 0 | 1 << 2 | 1 << 4 | 1 << 6 | 1 << 7 | 1 << 11);
        if result == 0 {
            flags |= 1 << 6;
        }
        if (result & 0x8000_0000) != 0 {
            flags |= 1 << 7;
        }
        if (result as u8).count_ones() % 2 == 0 {
            flags |= 1 << 2;
        }
        if cf {
            flags |= 1 << 0;
        }
        if of {
            flags |= 1 << 11;
        }
        if af {
            flags |= 1 << 4;
        }
        machine.registers.set_flags(flags);
    } else {
        log::error!("Memory operand in SUB r32, r/m32 not supported yet");
        machine.halted = true;
    }
}


pub fn add_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
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
    machine.registers.set_flags(flags::compute_flags_u32(result, cf, of, af));

    machine.log_instruction(csip, &bytes).ok();
}

pub fn add_eax_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm32 = machine.read_u32(machine.registers.cs(), machine.registers.ip());
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
    
    let mut flags = 0u16;
    if cf { flags |= 1 << 0; }
    if pf { flags |= 1 << 2; }
    if af { flags |= 1 << 4; }
    if zf { flags |= 1 << 6; }
    if sf { flags |= 1 << 7; }
    if of { flags |= 1 << 11; }
    machine.registers.set_flags(flags);
    
    machine.registers.set_eax(result);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn cmp_eax_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm32 = machine.read_u32(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(4)); // продвигаем на 4 байта (imm32)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm32.to_le_bytes());
    
    let eax = machine.registers.eax();
    
    // Вычисление флагов как при вычитании
    let result = eax.wrapping_sub(imm32);
    
    // Флаг переноса (CF): 1 если беззнаковое переполнение (EAX < imm32)
    let cf = eax < imm32;
    
    // Флаг переполнения (OF): знаковое переполнение при вычитании
    let eax_sign = (eax as i32) < 0;
    let imm_sign = (imm32 as i32) < 0;
    let result_sign = (result as i32) < 0;
    let of = (eax_sign != imm_sign) && (eax_sign != result_sign);
    
    // Флаг вспомогательного переноса (AF): перенос из бита 3 в бит 4
    let af = ((eax & 0x0F) as i32) < ((imm32 & 0x0F) as i32);
    
    // Установка флагов
    machine.registers.set_flags(flags::compute_flags_u32(result, cf, of, af));
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn test_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
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
    machine.registers.set_flags(flags::compute_logical_flags_u32(result));
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn cmp_r32_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
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

    let dst_val = machine.read_reg32(modrm.reg_field);
    let res = (dst_val as i64) - (src_val as i64);
    let result = res as u32;
    let cf = dst_val < src_val;
    let af = (dst_val & 0x0F) < (src_val & 0x0F);
    let of = (((dst_val ^ src_val) & 0x8000_0000) != 0) && (((dst_val ^ result) & 0x8000_0000) != 0);

    machine.registers.set_flags(flags::compute_flags_u32(result, cf, of, af));
    machine.log_instruction(csip, &bytes).ok();
}

pub fn cdq(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
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
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
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
    let imm32 = machine.read_u32(machine.registers.cs(), machine.registers.ip());
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
    flags = (flags & !(1 << 0)) | (overflow as u16);
    flags = (flags & !(1 << 11)) | ((overflow as u16) << 11);
    machine.registers.set_flags(flags);
    
    // Сохраняем усечённый результат в регистр назначения
    machine.write_reg32(modrm.reg_field, result_u32);
    
    machine.log_instruction(csip, &bytes).ok();
}

/// DEC r/m32 — Decrement doubleword by 1 (32-bit)
pub fn dec_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    let old_cf = machine.registers.flags() & 1;
    
    if modrm.is_register_mode() {
        let old_val = machine.read_reg32(modrm.rm_field);
        let new_val = old_val.wrapping_sub(1);
        machine.write_reg32(modrm.rm_field, new_val);
        
        let cf = old_val == 0;
        let of = old_val == 0x80000000;
        let af = (old_val & 0x0F) == 0;
        
        let mut flags = flags::compute_flags_u32(new_val, cf, of, af);
        flags = (flags & !1) | (old_cf);
        machine.registers.set_flags(flags as u16);
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        
        let old_val = machine.read_phys_u32(addr);
        let new_val = old_val.wrapping_sub(1);
        machine.write_phys_u32(addr, new_val);
        
        let cf = old_val == 0;
        let of = old_val == 0x80000000;
        let af = (old_val & 0x0F) == 0;
        
        let mut flags = flags::compute_flags_u32(new_val, cf, of, af);
        flags = (flags & !1) | (old_cf);
        machine.registers.set_flags(flags as u16);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

/// INC r/m32 — Increment doubleword by 1 (32-bit)
pub fn inc_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    let old_cf = machine.registers.flags() & 1;
    
    if modrm.is_register_mode() {
        let old_val = machine.read_reg32(modrm.rm_field);
        let new_val = old_val.wrapping_add(1);
        machine.write_reg32(modrm.rm_field, new_val);
        
        let cf = old_val == 0xFFFFFFFF;
        let of = old_val == 0x7FFFFFFF;
        let af = (old_val & 0x0F) == 0x0F;
        
        let mut flags = flags::compute_flags_u32(new_val, cf, of, af);
        flags = (flags & !1) | (old_cf);
        machine.registers.set_flags(flags as u16);
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        
        let old_val = machine.read_phys_u32(addr);
        let new_val = old_val.wrapping_add(1);
        machine.write_phys_u32(addr, new_val);
        
        let cf = old_val == 0xFFFFFFFF;
        let of = old_val == 0x7FFFFFFF;
        let af = (old_val & 0x0F) == 0x0F;
        
        let mut flags = flags::compute_flags_u32(new_val, cf, of, af);
        flags = (flags & !1) | (old_cf);
        machine.registers.set_flags(flags as u16);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}