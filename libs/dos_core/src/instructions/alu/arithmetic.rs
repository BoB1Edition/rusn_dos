use crate::{DosMachine, flags, modrm::ModRm};

pub fn add_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

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
    let dst_val = machine.read_reg8(modrm.rm_field); // приёмник
    let res = (dst_val as u16) + (src_val as u16);
    let result = res as u8;

    let cf = res > 0xFF;
    let af = ((dst_val & 0x0F) + (src_val & 0x0F)) > 0x0F;
    let of = (((dst_val ^ src_val) & 0x80) == 0) && ((dst_val ^ result) & 0x80) != 0;

    machine.write_reg8(modrm.rm_field, result);
    machine
        .registers
        .set_flags(flags::compute_flags_u8(result, cf, of, af));
    machine.log_instruction(csip, &bytes).ok();
}

pub fn add_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
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
    do_add_16(machine, modrm.rm_field, src_val);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn add_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
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
    do_add_16(machine, modrm.reg_field, src_val);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn add_al_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm8 = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None); // продвигаем на 1 байт (imm8)
    let mut bytes = prev.to_vec();
    bytes.push(imm8);
    let al = machine.registers.al();
    let result = (al as u16) + (imm8 as u16);
    let result8 = result as u8;
    let cf = result > 0xFF;
    let zf = result8 == 0;
    let sf = (result8 & 0x80) != 0;
    let af = ((al & 0x0F) + (imm8 & 0x0F)) > 0x0F;
    let pf = result8.count_ones() % 2 == 0;
    let al_sign = (al as i8) < 0;
    let imm_sign = (imm8 as i8) < 0;
    let result_sign = (result8 as i8) < 0;
    let of = (al_sign == imm_sign) && (result_sign != al_sign);
    let mut flags = 0u16;
    if cf { flags |= 1 << 0; }   // CF
    if pf { flags |= 1 << 2; }   // PF
    if af { flags |= 1 << 4; }   // AF
    if zf { flags |= 1 << 6; }   // ZF
    if sf { flags |= 1 << 7; }   // SF
    if of { flags |= 1 << 11; }  // OF
    machine.registers.set_flags(flags);
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
    machine
        .registers
        .set_flags(flags::compute_flags_u16(result, cf, of, af));
}

pub fn add_ax_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm16 = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(2));
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());
    
    let ax = machine.registers.ax();
    let res = (ax as u32) + (imm16 as u32);
    let result = res as u16;
    
    // Установка флагов
    let cf = res > 0xFFFF;
    let zf = result == 0;
    let sf = (result & 0x8000) != 0;
    let af = ((ax & 0x0F) + (imm16 & 0x0F)) > 0x0F;
    let pf = result.count_ones() % 2 == 0;
    let of = (((ax ^ imm16) & 0x8000) == 0) && ((ax ^ result) & 0x8000) != 0;
    
    let mut flags = 0u16;
    if cf { flags |= 1 << 0; }
    if pf { flags |= 1 << 2; }
    if af { flags |= 1 << 4; }
    if zf { flags |= 1 << 6; }
    if sf { flags |= 1 << 7; }
    if of { flags |= 1 << 11; }
    machine.registers.set_flags(flags);
    
    machine.registers.set_ax(result);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn cmp_ax_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm16 = machine.read_u16(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(Some(2)); // продвигаем на 2 байта (imm16)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());
    
    let ax = machine.registers.ax();
    
    // Вычисление флагов как при вычитании (беззнаковое и знаковое)
    let result = ax.wrapping_sub(imm16);
    
    // Флаг переноса (CF): 1 если беззнаковое переполнение (AX < imm16)
    let cf = ax < imm16;
    
    // Флаг переполнения (OF): знаковое переполнение при вычитании
    let ax_sign = (ax as i16) < 0;
    let imm_sign = (imm16 as i16) < 0;
    let result_sign = (result as i16) < 0;
    let of = (ax_sign != imm_sign) && (ax_sign != result_sign);
    
    // Флаг вспомогательного переноса (AF): перенос из бита 3 в бит 4
    let af = ((ax & 0x0F) as i16) < ((imm16 & 0x0F) as i16);
    
    // Установка флагов
    machine.registers.set_flags(flags::compute_flags_u16(result, cf, of, af));
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn sbb_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Источник: регистр из reg_field
    let src_val = machine.read_reg8(modrm.reg_field);
    
    // Текущее значение флага переноса (заём)
    let cf = (machine.registers.flags() & 1) != 0;
    let borrow = if cf { 1u16 } else { 0u16 };
    
    // Приёмник: r/m8 (регистр или память)
    if modrm.is_register_mode() {
        // SBB reg8, reg8
        let dst_val = machine.read_reg8(modrm.rm_field) as u16;
        let src_extended = src_val as u16 + borrow;
        
        // Вычисление результата и флагов
        let (result_u16, did_overflow) = dst_val.overflowing_sub(src_extended);
        let result = result_u16 as u8;
        
        // Флаг переноса (CF): 1 если было беззнаковое переполнение
        let new_cf = did_overflow;
        
        // Флаг переполнения (OF): знаковое переполнение при вычитании
        let dst_sign = (dst_val as i8) < 0;
        let src_sign = ((src_extended & 0xFF) as i8) < 0;
        let result_sign = (result as i8) < 0;
        let new_of = (dst_sign != src_sign) && (dst_sign != result_sign);
        
        // Флаг вспомогательного переноса (AF): заём из бита 3 в бит 4
        let dst_low = dst_val & 0x0F;
        let src_low = src_extended & 0x0F;
        let new_af = dst_low < src_low;
        
        // Установка результата и флагов
        machine.write_reg8(modrm.rm_field, result);
        machine.registers.set_flags(flags::compute_flags_u8(result, new_cf, new_of, new_af));
    } else {
        // SBB [mem], reg8
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
        machine.registers.set_flags(flags::compute_flags_u8(result, new_cf, new_of, new_af));
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn sub_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    machine.log_instruction(csip, &bytes).ok();

    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        // SUB reg16, reg16
        let dst_val = machine.read_reg16(modrm.reg_field); // приёмник
        let src_val = machine.read_reg16(modrm.rm_field); // источник
        let res = (dst_val as i32) - (src_val as i32);
        let result = res as u16;
        let cf = (dst_val as u32) < (src_val as u32);
        let af = (dst_val & 0x0F) < (src_val & 0x0F);
        let of = (((dst_val ^ src_val) & 0x8000) != 0) && (((dst_val ^ result) & 0x8000) != 0);
        machine.write_reg16(modrm.reg_field, result);
        machine
            .registers
            .set_flags(flags::compute_flags_u16(result, cf, of, af));
    } else {
        log::error!("Memory operand in SUB r16, r/m16 not supported yet");
        machine.halted = true;
    }
}

pub fn cmp_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        machine.read_phys_u16(addr)
    };

    let dst_val = machine.read_reg16(modrm.reg_field);
    let res = (dst_val as i32) - (src_val as i32);
    let result = res as u16;
    let cf = (dst_val as u32) < (src_val as u32);
    let af = (dst_val & 0x0F) < (src_val & 0x0F);
    let of = (((dst_val ^ src_val) & 0x8000) != 0) && (((dst_val ^ result) & 0x8000) != 0);

    machine.registers.set_flags(flags::compute_flags_u16(result, cf, of, af));
    machine.log_instruction(csip, &bytes).ok();
}

pub fn cmp_al_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let imm8 = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None); // продвигаем на 1 байт (imm8)
    let mut bytes = prev.to_vec();
    bytes.push(imm8);
    
    let al = machine.registers.al();
    
    // Вычисление флагов как при вычитании (беззнаковое и знаковое)
    let result = al.wrapping_sub(imm8);
    
    // Флаг переноса (CF): 1 если беззнаковое переполнение (AL < imm8)
    let cf = al < imm8;
    
    // Флаг переполнения (OF): знаковое переполнение при вычитании
    // Возникает когда: 
    //   (AL >= 0 и imm8 < 0) → результат < 0  ИЛИ
    //   (AL < 0 и imm8 >= 0) → результат >= 0
    let al_sign = (al as i8) < 0;
    let imm_sign = (imm8 as i8) < 0;
    let result_sign = (result as i8) < 0;
    let of = (al_sign != imm_sign) && (al_sign != result_sign);
    
    // Флаг вспомогательного переноса (AF): перенос из бита 3 в бит 4
    let af = ((al & 0x0F) as i8) < ((imm8 & 0x0F) as i8);
    
    // Установка флагов
    machine.registers.set_flags(flags::compute_flags_u8(result, cf, of, af));
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn inc_bp(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let bytes = prev.to_vec();
    
    let bp = machine.registers.bp();
    let result = bp.wrapping_add(1);
    
    // Флаг вспомогательного переноса (AF): перенос из бита 3 в бит 4
    let af = ((bp & 0x0F) + 1) > 0x0F;
    
    // Флаг переполнения (OF): знаковое переполнение при инкременте
    // Возникает при переходе 0x7FFF → 0x8000 (32767 → -32768)
    let of = bp == 0x7FFF;
    
    // Сохраняем текущий флаг переноса (CF) — он НЕ изменяется
    let current_cf = (machine.registers.flags() & 1) != 0;
    
    // Установка результата и флагов
    machine.registers.set_bp(result);
    machine.registers.set_flags(flags::compute_flags_u16(result, current_cf, of, af));
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn dec_ax(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    bytes.push(0x48); // опкод DEC AX
    
    let ax = machine.registers.ax();
    let result = ax.wrapping_sub(1);
    
    // Флаг вспомогательного переноса (AF): заём из бита 3 в бит 4
    let af = (ax & 0x0F) == 0;
    
    // Флаг переполнения (OF): знаковое переполнение при декременте
    // Возникает при переходе 0x8000 → 0x7FFF (-32768 → 32767)
    let of = ax == 0x8000;
    
    // Сохраняем текущий флаг переноса (CF) — он НЕ изменяется
    let current_cf = (machine.registers.flags() & 1) != 0;
    
    // Установка результата и флагов
    machine.registers.set_ax(result);
    machine.registers.set_flags(flags::compute_flags_u16(result, current_cf, of, af));
    
    machine.log_instruction(csip, &bytes).ok();
}