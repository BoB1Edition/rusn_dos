// Ver: 1 File: ./libs/dos_core/src/instructions/bcd.rs

use crate::{DosMachine, flags};

/// DAS — Decimal Adjust AL after Subtraction (опкод 0x2F)
/// Корректирует регистр AL для получения правильного результата в упакованном BCD
pub fn das(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let mut bytes = prev.to_vec();
    bytes.push(0x2F); // опкод DAS
    
    let al = machine.registers.al();
    let flags = machine.registers.flags();
    let af = (flags & (flags::AF)) != 0; // Auxiliary Flag (бит 4)
    let cf = (flags & 1) != 0;        // Carry Flag (бит 0)
    
    let mut new_al = al;
    let mut new_cf = cf;
    let mut new_af = af;
    
    // Шаг 1: коррекция младшего полубайта (биты 0-3)
    let lower_nibble = al & 0x0F;
    if af || lower_nibble > 9 {
        new_al = new_al.wrapping_sub(6);
        new_af = true;
    }
    
    // Шаг 2: коррекция старшего полубайта (биты 4-7)
    let upper_nibble = (new_al >> 4) & 0x0F;
    if cf || upper_nibble > 9 {
        new_al = new_al.wrapping_sub(0x60);
        new_cf = true;
    }
    
    // Установка результата в AL
    machine.registers.set_al(new_al);
    
    // Установка флагов
    let mut new_flags = flags & !(1 | (flags::PF) | (flags::AF) | (flags::ZF) | (flags::SF));
    if new_cf { new_flags |= flags::CF; }  // CF
    if new_al.count_ones() % 2 == 0 { new_flags |= flags::PF; }  // PF
    if new_af { new_flags |= flags::AF; }  // AF
    if new_al == 0 { new_flags |= flags::ZF; }  // ZF
    if (new_al & 0x80) != 0 { new_flags |= flags::SF; }  // SF
    // OF не определён по спецификации — сохраняем предыдущее значение
    
    machine.registers.set_flags(new_flags);
    machine.log_instruction(csip, &bytes).ok();
}

/// DAA — Decimal Adjust after Addition (опкод 0x27)
/// Корректирует результат сложения BCD в регистре AL
pub(crate) fn daa(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let bytes = prev.to_vec();

    let mut al = machine.registers.al();
    let mut flags = machine.registers.flags();
    let cf_old = (flags & flags::CF) != 0;
    let af_old = (flags & flags::AF) != 0;

    let mut cf_new = false;
    let mut af_new = false;

    // Шаг 1: корректировка младшей тетрады
    if (al & 0x0F) > 9 || af_old {
        al = al.wrapping_add(6);
        af_new = true;
        // Если был перенос из младшей тетрады, устанавливаем CF
        if (al < 6) || cf_old {
            cf_new = true;
        }
    }

    // Шаг 2: корректировка старшей тетрады
    if al > 0x99 || cf_old {
        al = al.wrapping_add(0x60);
        cf_new = true;
    }

    // Записываем результат
    machine.registers.set_al(al);

    // Устанавливаем флаги SF, ZF, PF
    let sf = (al & 0x80) != 0;
    let zf = al == 0;
    let pf = (al.count_ones() % 2) == 0;

    // Обновляем флаги
    flags &= !(flags::CF | flags::AF | flags::SF | flags::ZF | flags::PF);
    if cf_new {
        flags |= flags::CF;
    }
    if af_new {
        flags |= flags::AF;
    }
    if sf {
        flags |= flags::SF;
    }
    if zf {
        flags |= flags::ZF;
    }
    if pf {
        flags |= flags::PF;
    }

    machine.registers.set_flags(flags);
    machine.log_instruction(csip, &bytes).ok();
}