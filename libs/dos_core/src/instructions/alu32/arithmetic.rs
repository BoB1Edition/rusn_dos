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
