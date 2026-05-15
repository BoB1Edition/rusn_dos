// Ver: 1

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
