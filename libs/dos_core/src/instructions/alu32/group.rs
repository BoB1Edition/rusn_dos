// Ver: 1
use crate::{DosMachine, flags, modrm::ModRm};


pub fn group_x83_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    let imm8 = machine.read_u8(
        machine.registers.cs(),
        machine.registers.ip().wrapping_add(1),
    );
    machine.registers.step(Some(2));
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    bytes.push(imm8);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // Расширяем imm8 до i8, затем до i32 со знаком (sign-extend)
    let imm32 = (imm8 as i8) as i32 as u32;
    
    if modrm.is_register_mode() {
        // Операция над регистром
        let dst_val = machine.read_reg32(modrm.rm_field);
        let (result, flags) = perform_group_x83_operation_32(modrm.reg_field, dst_val, imm32, machine.registers.flags());
        if modrm.reg_field != 7 { // CMP (reg_field=7) не сохраняет результат
            machine.write_reg32(modrm.rm_field, result);
        }
        machine.registers.set_flags(flags);
    } else {
        // Операция над памятью
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let dst_val = machine.read_phys_u32(addr);
        let (result, flags) = perform_group_x83_operation_32(modrm.reg_field, dst_val, imm32, machine.registers.flags());
        if modrm.reg_field != 7 { // CMP не сохраняет результат
            machine.write_phys_u32(addr, result);
        }
        machine.registers.set_flags(flags);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

fn perform_group_x83_operation_32(op_field: u8, dst_val: u32, imm: u32, flags: u16) -> (u32, u16) {
    match op_field {
        0 => { // ADD r/m32, imm8 (sign-extended)
            let res = dst_val as u64 + imm as u64;
            let result = res as u32;
            let cf = res > 0xFFFFFFFF;
            let af = ((dst_val & 0x0F) + (imm & 0x0F)) > 0x0F;
            let of = (((dst_val ^ imm) & 0x8000_0000) == 0) && ((dst_val ^ result) & 0x8000_0000) != 0;
            (result, flags::compute_flags_u32(result, cf, of, af))
        }
        1 => { // OR r/m32, imm8
            let result = dst_val | imm;
            (result, flags::compute_logical_flags_u32(result))
        }
        2 => { // ADC r/m32, imm8
            let carry_in = (flags & 1) != 0;
            let mut res = dst_val as u64 + imm as u64;
            if carry_in {
                res += 1;
            }
            let result = res as u32;
            let cf = res > 0xFFFFFFFF;
            let af = ((dst_val & 0x0F) + (imm & 0x0F) + if carry_in { 1 } else { 0 }) > 0x0F;
            let of = (((dst_val ^ imm) & 0x8000_0000) == 0) && ((dst_val ^ result) & 0x8000_0000) != 0;
            (result, flags::compute_flags_u32(result, cf, of, af))
        }
        3 => { // SBB r/m32, imm8
            let borrow_in = (flags & 1) != 0;
            let imm_with_borrow = imm as u64 + if borrow_in { 1 } else { 0 };
            let cf = (dst_val as u64) < imm_with_borrow;
            let res = (dst_val as u64).wrapping_sub(imm_with_borrow);
            let result = res as u32;
            let af = (dst_val & 0x0F) < ((imm & 0x0F) + if borrow_in { 1 } else { 0 });
            let of = (((dst_val ^ imm) & 0x8000_0000) != 0) && (((dst_val ^ result) & 0x8000_0000) != 0);
            (result, flags::compute_flags_u32(result, cf, of, af))
        }
        4 => { // AND r/m32, imm8
            let result = dst_val & imm;
            (result, flags::compute_logical_flags_u32(result))
        }
        5 => { // SUB r/m32, imm8
            let res = (dst_val as u64).wrapping_sub(imm as u64);
            let result = res as u32;
            let cf = dst_val < imm;
            let af = (dst_val & 0x0F) < (imm & 0x0F);
            let of = (((dst_val ^ imm) & 0x8000_0000) != 0) && (((dst_val ^ result) & 0x8000_0000) != 0);
            (result, flags::compute_flags_u32(result, cf, of, af))
        }
        6 => { // XOR r/m32, imm8
            let result = dst_val ^ imm;
            (result, flags::compute_logical_flags_u32(result))
        }
        7 => { // CMP r/m32, imm8 — как SUB, но не сохраняем результат
            let res = (dst_val as u64).wrapping_sub(imm as u64);
            let result = res as u32;
            let cf = dst_val < imm;
            let af = (dst_val & 0x0F) < (imm & 0x0F);
            let of = (((dst_val ^ imm) & 0x8000_0000) != 0) && (((dst_val ^ result) & 0x8000_0000) != 0);
            let flags = flags::compute_flags_u32(result, cf, of, af);
            (dst_val, flags) // Возвращаем исходное значение (не сохраняем результат)
        }
        _ => unreachable!(),
    }
}

pub fn group_f7_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let reg_field = modrm.reg_field;
    
    // Для TEST (reg_field 0/1) требуется дополнительное слово imm32
    let imm32 = if reg_field == 0 || reg_field == 1 {
        let imm = machine.read_u32(machine.registers.cs(), machine.registers.ip());
        machine.registers.step(Some(4));
        bytes.extend_from_slice(&imm.to_le_bytes());
        Some(imm)
    } else {
        None
    };
    
    if modrm.is_register_mode() {
        group_f7_register_32(machine, reg_field, modrm.rm_field, imm32);
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        group_f7_memory_32(machine, reg_field, addr, imm32);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

fn group_f7_register_32(machine: &mut DosMachine, reg_field: u8, rm_field: u8, imm32: Option<u32>) {
    let value = machine.read_reg32(rm_field);
    
    match reg_field {
        0 | 1 => {
            // TEST r32, imm32
            let imm = imm32.unwrap();
            let result = value & imm;
            machine.registers.set_flags(flags::compute_logical_flags_u32(result));
        }
        2 => {
            // NOT r32
            let result = !value;
            machine.write_reg32(rm_field, result);
            // Флаги НЕ изменяются!
        }
        3 => {
            // NEG r32
            let result = value.wrapping_neg();
            let cf = value != 0;
            let af = (value & 0x0F) != 0;
            let of = value == 0x8000_0000;
            machine.write_reg32(rm_field, result);
            machine.registers.set_flags(flags::compute_flags_u32(result, cf, of, af));
        }
        4 => {
            // MUL r32 (беззнаковое 32×32→64)
            let src = value;
            let eax = machine.registers.eax();
            let product = (eax as u64) * (src as u64);
            machine.registers.set_eax((product & 0xFFFF_FFFF) as u32);
            machine.registers.set_edx((product >> 32) as u32);
            let cf_of = product > 0xFFFF_FFFF;
            let mut flags = machine.registers.flags();
            if cf_of {
                flags |= 1 << 0 | 1 << 11;
            } else {
                flags &= !(1 << 0 | 1 << 11);
            }
            machine.registers.set_flags(flags);
        }
        5 => {
            // IMUL r32 (знаковое 32×32→64)
            let src = value as i32 as i64;
            let eax = machine.registers.eax() as i32 as i64;
            let product = eax * src;
            machine.registers.set_eax((product & 0xFFFF_FFFF) as u32);
            machine.registers.set_edx(((product >> 32) & 0xFFFF_FFFF) as u32);
            let cf_of = product < -2_147_483_648 || product > 2_147_483_647;
            let mut flags = machine.registers.flags();
            if cf_of {
                flags |= 1 << 0 | 1 << 11;
            } else {
                flags &= !(1 << 0 | 1 << 11);
            }
            machine.registers.set_flags(flags);
        }
        6 => {
            // DIV r32 (беззнаковое 64/32→32+32)
            let divisor = value;
            if divisor == 0 {
                log::error!("Division by zero in DIV r32");
                machine.halted = true;
                return;
            }
            let edx = machine.registers.edx() as u64;
            let eax = machine.registers.eax() as u64;
            let dividend = (edx << 32) | eax;
            let quotient = dividend / divisor as u64;
            let remainder = dividend % divisor as u64;
            if quotient > 0xFFFF_FFFF {
                log::error!("Divide overflow in DIV r32 (quotient > 0xFFFFFFFF)");
                machine.halted = true;
                return;
            }
            machine.registers.set_eax(quotient as u32);
            machine.registers.set_edx(remainder as u32);
        }
        7 => {
            // IDIV r32 (знаковое 64/32→32+32)
            let divisor = value as i32;
            if divisor == 0 {
                log::error!("Division by zero in IDIV r32");
                machine.halted = true;
                return;
            }
            let edx = machine.registers.edx() as i64;
            let eax = machine.registers.eax() as i64;
            let dividend = (edx << 32) | eax;
            // Особый случай: -2^63 / -1 вызывает переполнение
            if dividend == -9_223_372_036_854_775_808 && divisor == -1 {
                log::error!("Divide overflow in IDIV r32 (-2^63 / -1)");
                machine.halted = true;
                return;
            }
            let quotient = dividend / divisor as i64;
            let remainder = dividend % divisor as i64;
            if quotient < -2_147_483_648 || quotient > 2_147_483_647 {
                log::error!("Divide overflow in IDIV r32 (quotient out of range)");
                machine.halted = true;
                return;
            }
            machine.registers.set_eax(quotient as u32);
            machine.registers.set_edx(remainder as u32);
        }
        _ => unreachable!(),
    }
}

fn group_f7_memory_32(machine: &mut DosMachine, reg_field: u8, addr: u32, imm32: Option<u32>) {
    let value = machine.read_phys_u32(addr);
    
    match reg_field {
        0 | 1 => {
            // TEST [mem], imm32
            let imm = imm32.unwrap();
            let result = value & imm;
            machine.registers.set_flags(flags::compute_logical_flags_u32(result));
        }
        2 => {
            // NOT [mem]
            let result = !value;
            machine.write_phys_u32(addr, result);
            // Флаги НЕ изменяются!
        }
        3 => {
            // NEG [mem]
            let result = value.wrapping_neg();
            let cf = value != 0;
            let af = (value & 0x0F) != 0;
            let of = value == 0x8000_0000;
            machine.write_phys_u32(addr, result);
            machine.registers.set_flags(flags::compute_flags_u32(result, cf, of, af));
        }
        4 => {
            // MUL [mem] (беззнаковое)
            let src = value;
            let eax = machine.registers.eax();
            let product = (eax as u64) * (src as u64);
            machine.registers.set_eax((product & 0xFFFF_FFFF) as u32);
            machine.registers.set_edx((product >> 32) as u32);
            let cf_of = product > 0xFFFF_FFFF;
            let mut flags = machine.registers.flags();
            if cf_of {
                flags |= 1 << 0 | 1 << 11;
            } else {
                flags &= !(1 << 0 | 1 << 11);
            }
            machine.registers.set_flags(flags);
        }
        5 => {
            // IMUL [mem] (знаковое)
            let src = value as i32 as i64;
            let eax = machine.registers.eax() as i32 as i64;
            let product = eax * src;
            machine.registers.set_eax((product & 0xFFFF_FFFF) as u32);
            machine.registers.set_edx(((product >> 32) & 0xFFFF_FFFF) as u32);
            let cf_of = product < -2_147_483_648 || product > 2_147_483_647;
            let mut flags = machine.registers.flags();
            if cf_of {
                flags |= 1 << 0 | 1 << 11;
            } else {
                flags &= !(1 << 0 | 1 << 11);
            }
            machine.registers.set_flags(flags);
        }
        6 => {
            // DIV [mem] (беззнаковое)
            let divisor = value;
            if divisor == 0 {
                log::error!("Division by zero in DIV [mem]");
                machine.halted = true;
                return;
            }
            let edx = machine.registers.edx() as u64;
            let eax = machine.registers.eax() as u64;
            let dividend = (edx << 32) | eax;
            let quotient = dividend / divisor as u64;
            let remainder = dividend % divisor as u64;
            if quotient > 0xFFFF_FFFF {
                log::error!("Divide overflow in DIV [mem]");
                machine.halted = true;
                return;
            }
            machine.registers.set_eax(quotient as u32);
            machine.registers.set_edx(remainder as u32);
        }
        7 => {
            // IDIV [mem] (знаковое)
            let divisor = value as i32;
            if divisor == 0 {
                log::error!("Division by zero in IDIV [mem]");
                machine.halted = true;
                return;
            }
            let edx = machine.registers.edx() as i64;
            let eax = machine.registers.eax() as i64;
            let dividend = (edx << 32) | eax;
            if dividend == -9_223_372_036_854_775_808 && divisor == -1 {
                log::error!("Divide overflow in IDIV [mem] (-2^63 / -1)");
                machine.halted = true;
                return;
            }
            let quotient = dividend / divisor as i64;
            let remainder = dividend % divisor as i64;
            if quotient < -2_147_483_648 || quotient > 2_147_483_647 {
                log::error!("Divide overflow in IDIV [mem]");
                machine.halted = true;
                return;
            }
            machine.registers.set_eax(quotient as u32);
            machine.registers.set_edx(remainder as u32);
        }
        _ => unreachable!(),
    }
}