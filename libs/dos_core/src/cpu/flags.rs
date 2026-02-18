// Ver: 2
//! Централизованная обработка флагов процессора x86 (FLAGS register)
//! 
//! Формат регистра FLAGS (16 бит):
//!   15  14  13  12  11  10   9   8   7   6   5   4   3   2   1   0
//!   ─── ─── ─── ─── ─── ─── ─── ─── ─── ─── ─── ─── ─── ─── ─── ───
//!   ??  ??  ??  ??  OF  DF  IF  TF  SF  ZF  ??  AF  ??  PF  ??  CF
//! 
//! Ключевые флаги:
//!   CF (bit 0)  — Carry Flag      — перенос/заём
//!   PF (bit 2)  — Parity Flag     — чётность младшего байта результата
//!   AF (bit 4)  — Auxiliary Flag  — перенос из бит 3→4 (для BCD)
//!   ZF (bit 6)  — Zero Flag       — результат равен нулю
//!   SF (bit 7)  — Sign Flag       — знак результата (старший бит)
//!   OF (bit 11) — Overflow Flag   — переполнение знаковой арифметики

/// Битовые маски флагов
pub const CF: u16 = 1 << 0;   // Carry Flag
pub const PF: u16 = 1 << 2;   // Parity Flag
pub const AF: u16 = 1 << 4;   // Auxiliary Flag
pub const ZF: u16 = 1 << 6;   // Zero Flag
pub const SF: u16 = 1 << 7;   // Sign Flag
pub const OF: u16 = 1 << 11;  // Overflow Flag
pub const IF: u16 = 1 << 9;
pub const DF: u16 = 1 << 10;

#[inline]
pub fn test_df(flags: u16) -> bool {
    test_flag(flags, DF)
}

#[inline]
pub fn set_df(flags: &mut u16) {
    *flags |= DF;
}

#[inline]
pub fn test_if(flags: u16) -> bool {
    test_flag(flags, IF)
}

/// Вычисляет все арифметические флаги для 8-битного результата
#[inline]
pub fn compute_flags_u8(result: u8, cf: bool, of: bool, af: bool) -> u16 {
    compute_flags_impl(result as u32, 8, cf, of, af)
}

/// Вычисляет все арифметические флаги для 16-битного результата
#[inline]
pub fn compute_flags_u16(result: u16, cf: bool, of: bool, af: bool) -> u16 {
    compute_flags_impl(result as u32, 16, cf, of, af)
}

/// Вычисляет все арифметические флаги для 32-битного результата
#[inline]
pub fn compute_flags_u32(result: u32, cf: bool, of: bool, af: bool) -> u16 {
    compute_flags_impl(result, 32, cf, of, af)
}

/// Внутренняя реализация вычисления флагов
#[inline]
fn compute_flags_impl(value: u32, width_bits: u32, cf: bool, of: bool, af: bool) -> u16 {
    let mut flags = 0u16;
    
    // ZF: Zero Flag — результат равен нулю
    if value == 0 {
        flags |= ZF;
    }
    
    // SF: Sign Flag — старший бит результата
    let sign_bit = 1u32 << (width_bits - 1);
    if (value & sign_bit) != 0 {
        flags |= SF;
    }
    
    // PF: Parity Flag — чётность младшего байта (чётное количество единиц)
    if (value as u8).count_ones() % 2 == 0 {
        flags |= PF;
    }
    
    // CF, OF, AF — передаются извне (вычисляются в арифметических операциях)
    if cf {
        flags |= CF;
    }
    if of {
        flags |= OF;
    }
    if af {
        flags |= AF;
    }
    
    flags
}

/// Вычисляет флаги для логических операций (AND, OR, XOR, TEST)
/// CF=0, OF=0, AF не определён (обычно не изменяется)
#[inline]
pub fn compute_logical_flags_u8(result: u8) -> u16 {
    compute_flags_u8(result, false, false, false)
}

/// Вычисляет флаги для логических операций (AND, OR, XOR, TEST)
#[inline]
pub fn compute_logical_flags_u16(result: u16) -> u16 {
    compute_flags_u16(result, false, false, false)
}

/// Вычисляет флаги для логических операций (AND, OR, XOR, TEST)
#[inline]
pub fn compute_logical_flags_u32(result: u32) -> u16 {
    compute_flags_u32(result, false, false, false)
}

/// Проверяет состояние флага в регистре FLAGS
#[inline]
pub fn test_flag(flags: u16, bit_mask: u16) -> bool {
    (flags & bit_mask) != 0
}

/// Устанавливает или сбрасывает флаг в регистре FLAGS
#[inline]
pub fn set_flag(flags: &mut u16, bit_mask: u16, value: bool) {
    if value {
        *flags |= bit_mask;
    } else {
        *flags &= !bit_mask;
    }
}

/// Проверяет флаг переноса (CF)
#[inline]
pub fn test_cf(flags: u16) -> bool {
    test_flag(flags, CF)
}

/// Проверяет флаг нуля (ZF)
#[inline]
pub fn test_zf(flags: u16) -> bool {
    test_flag(flags, ZF)
}

/// Проверяет флаг знака (SF)
#[inline]
pub fn test_sf(flags: u16) -> bool {
    test_flag(flags, SF)
}

/// Проверяет флаг переполнения (OF)
#[inline]
pub fn test_of(flags: u16) -> bool {
    test_flag(flags, OF)
}

#[inline]
pub fn test_pf(flags: u16) -> bool {
    test_flag(flags, PF)
}

/// Сбрасывает флаг направления (DF) — для инструкций строковых операций
#[inline]
pub fn clear_df(flags: &mut u16) {
    *flags &= !(1 << 10);
}

/// Устанавливает флаг прерываний (IF) — разрешение маскируемых прерываний
#[inline]
pub fn set_if(flags: &mut u16) {
    *flags |= 1 << 9;
}

/// Сбрасывает флаг прерываний (IF) — запрет маскируемых прерываний
#[inline]
pub fn clear_if(flags: &mut u16) {
    *flags &= !(1 << 9);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_flags_add_8bit() {
        // 0xFF + 1 = 0x00 с переносом
        let flags = compute_flags_u8(0x00, true, false, true);
        assert!(test_cf(flags));   // CF=1
        assert!(test_zf(flags));   // ZF=1
        assert!(!test_sf(flags));  // SF=0
        assert!(test_pf(flags));   // PF=1 (0x00 имеет 0 единиц → чётное)
        assert!(!test_of(flags));  // OF=0
    }

    #[test]
    fn test_compute_flags_overflow() {
        // 0x7F + 1 = 0x80 — переполнение знака (положительное → отрицательное)
        let flags = compute_flags_u8(0x80, false, true, false);
        assert!(!test_cf(flags));  // CF=0 (беззнаковое сложение без переноса)
        assert!(!test_zf(flags));  // ZF=0
        assert!(test_sf(flags));   // SF=1 (старший бит = 1)
        assert!(!test_pf(flags));  // PF=0 (0x80 = 10000000b → 1 единица → нечётное)
        assert!(test_of(flags));   // OF=1 (переполнение знака)
    }

    #[test]
    fn test_logical_flags() {
        // AND 0x0F, 0x0F = 0x0F
        let flags = compute_logical_flags_u8(0x0F);
        assert!(!test_cf(flags));  // CF=0 для логических операций
        assert!(!test_zf(flags));  // ZF=0 (результат ≠ 0)
        assert!(!test_sf(flags));  // SF=0 (старший бит = 0)
        assert!(!test_pf(flags));  // PF=0 (0x0F = 00001111b → 4 единицы → чётное? НЕТ: 4 % 2 = 0 → чётное → PF=1)
        assert!(!test_of(flags));  // OF=0 для логических операций
    }
}