// Ver: 1 File: crate/x86/src/fpu/f80.rs

//! Ядро 80-битного расширенного формата (x87 extended).
//!
//! Формат (80 бит):
//! - Бит 79     : знак
//! - Биты 78..64: экспонента (15 бит, смещение 16383)
//! - Бит 63     : целочисленный бит (J, явный — в отличие от IEEE-754)
//! - Биты 62..0 : дробь (63 бита)
//!
//! В памяти (little-endian, 10 байт):
//! - байты 0..7 : мантисса (бит 63 = J)
//! - байты 8..9 : знак + экспонента

use bus::Machine;

/// Смещение экспоненты.
pub const EXP_BIAS: u32 = 16383;

/// Максимальное значение смещённой экспоненты (все 15 бит = 1).
pub const EXP_MAX: u16 = 0x7FFF;

/// 80-битное расширенное число в упакованном виде.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct F80 {
    /// Бит 63 = целочисленный бит (J), биты 62..0 = дробь.
    pub mant: u64,
    /// Бит 15 = знак, биты 14..0 = смещённая экспонента.
    pub sign_exp: u16,
}

/// Классификация значения.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum F80Class {
    /// ±0: экспонента=0, мантисса=0
    Zero,
    /// Денормал: экспонента=0, J=0, мантисса≠0
    Subnormal,
    /// Unnormal (псевдо-денормал): экспонента=0, J=1, мантисса≠0
    Unnormal,
    /// Денормал с ненулевой экспонентой: 0<экспонента<0x7FFF, J=0
    Denormal,
    /// Нормальное число: 0<экспонента<0x7FFF, J=1
    Normal,
    /// ±∞: экспонента=0x7FFF, J=1, дробь=0
    Infinity,
    /// Тихий NaN: экспонента=0x7FFF, J=1, дробь≠0, бит 62 дроби = 1
    QNaN,
    /// Сигнальный NaN: экспонента=0x7FFF, J=1, дробь≠0, бит 62 дроби = 0
    SNaN,
    /// Псевдо-бесконечность: экспонента=0x7FFF, J=0, дробь=0 (не поддерживается)
    PseudoInfinity,
    /// Псевдо-NaN: экспонента=0x7FFF, J=0, дробь≠0 (не поддерживается)
    PseudoNaN,
}

impl F80 {
    // === Доступ к полям ===

    #[inline]
    pub fn is_denormal(&self) -> bool {
        let result = matches!(
            self.classify(),
            F80Class::Subnormal | F80Class::Unnormal | F80Class::Denormal
        );
        result
    }

    #[inline]
    pub fn sign(&self) -> bool {
        (self.sign_exp >> 15) & 1 != 0
    }

    #[inline]
    pub fn biased_exp(&self) -> u16 {
        self.sign_exp & EXP_MAX
    }

    #[inline]
    pub fn int_bit(&self) -> u64 {
        (self.mant >> 63) & 1
    }

    #[inline]
    pub fn fraction(&self) -> u64 {
        self.mant & 0x7FFF_FFFF_FFFF_FFFF
    }

    // === Конструкторы ===

    #[inline]
    pub fn from_parts(sign: bool, biased_exp: u16, mant: u64) -> Self {
        Self {
            mant,
            sign_exp: ((sign as u16) << 15) | (biased_exp & EXP_MAX),
        }
    }

    /// ±0.
    #[inline]
    pub fn zero(sign: bool) -> Self {
        Self::from_parts(sign, 0, 0)
    }

    /// ±1.0 (экспонента = 16383, J=1, дробь=0).
    #[inline]
    pub fn one(sign: bool) -> Self {
        Self::from_parts(sign, EXP_BIAS as u16, 1 << 63)
    }

    /// ±∞.
    #[inline]
    pub fn infinity(sign: bool) -> Self {
        Self::from_parts(sign, EXP_MAX, 1 << 63)
    }

    /// Тихий NaN (канонический).
    #[inline]
    pub fn qnan() -> Self {
        Self::from_parts(false, EXP_MAX, (1 << 63) | (1 << 62))
    }

    /// Сигнальный NaN (канонический).
    #[inline]
    pub fn snan() -> Self {
        Self::from_parts(false, EXP_MAX, (1 << 63) | 1)
    }

    // === Классификация ===

    /// Определяет класс значения.
    #[inline]
    pub fn classify(&self) -> F80Class {
        let exp = self.biased_exp();
        let mant = self.mant;
        let int_bit = self.int_bit();
        let frac = self.fraction();

        if exp == 0 {
            if mant == 0 {
                F80Class::Zero
            } else if int_bit == 0 {
                F80Class::Subnormal
            } else {
                F80Class::Unnormal
            }
        } else if exp == EXP_MAX {
            if int_bit == 1 {
                if frac == 0 {
                    F80Class::Infinity
                } else if (frac >> 62) & 1 != 0 {
                    F80Class::QNaN
                } else {
                    F80Class::SNaN
                }
            } else if frac == 0 {
                F80Class::PseudoInfinity
            } else {
                F80Class::PseudoNaN
            }
        } else if int_bit == 1 {
            F80Class::Normal
        } else {
            F80Class::Denormal
        }
    }

    /// Является ли значение конечным числом (не NaN и не бесконечность).
    #[inline]
    pub fn is_finite(&self) -> bool {
        let result = matches!(
            self.classify(),
            F80Class::Zero | F80Class::Subnormal | F80Class::Unnormal
                | F80Class::Denormal | F80Class::Normal
        );
        result
    }

    /// Является ли значение NaN (любым).
    #[inline]
    pub fn is_nan(&self) -> bool {
        let result = matches!(
            self.classify(),
            F80Class::QNaN | F80Class::SNaN | F80Class::PseudoNaN
        );
        result
    }

    // === Память ===

    /// Читает 80-битное число из памяти (10 байт, little-endian).
    pub fn read_from(machine: &dyn Machine, addr: u32) -> Self {
        let lo = machine.read_mem_u32(addr);
        let hi = machine.read_mem_u32(addr.wrapping_add(4));
        let mant = (lo as u64) | ((hi as u64) << 32);
        let sign_exp = machine.read_mem_u16(addr.wrapping_add(8));
        Self { mant, sign_exp }
    }

    /// Записывает 80-битное число в память (10 байт, little-endian).
    pub fn write_to(&self, machine: &mut dyn Machine, addr: u32) {
        machine.write_mem_u32(addr, self.mant as u32);
        machine.write_mem_u32(addr.wrapping_add(4), (self.mant >> 32) as u32);
        machine.write_mem_u16(addr.wrapping_add(8), self.sign_exp);
    }

    // === Отладка ===

    /// Шестнадцатеричное представление для логов.
    pub fn hex(&self) -> String {
        let result = format!("{:04X}:{:016X}", self.sign_exp, self.mant);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bus::Motherboard;

    #[test]
    fn test_zero() {
        let z = F80::zero(false);
        assert_eq!(z.classify(), F80Class::Zero);
        assert!(!z.sign());

        let nz = F80::zero(true);
        assert_eq!(nz.classify(), F80Class::Zero);
        assert!(nz.sign());
    }

    #[test]
    fn test_one() {
        let one = F80::one(false);
        assert_eq!(one.classify(), F80Class::Normal);
        assert_eq!(one.biased_exp(), EXP_BIAS as u16);
        assert_eq!(one.int_bit(), 1);
        assert_eq!(one.fraction(), 0);
    }

    #[test]
    fn test_infinity() {
        let inf = F80::infinity(false);
        assert_eq!(inf.classify(), F80Class::Infinity);
        assert!(!inf.sign());

        let ninf = F80::infinity(true);
        assert_eq!(ninf.classify(), F80Class::Infinity);
        assert!(ninf.sign());
    }

    #[test]
    fn test_qnan() {
        let q = F80::qnan();
        assert_eq!(q.classify(), F80Class::QNaN);
        assert!(q.is_nan());
        assert!(!q.is_finite());
    }

    #[test]
    fn test_snan() {
        let s = F80::snan();
        assert_eq!(s.classify(), F80Class::SNaN);
        assert!(s.is_nan());
    }

    #[test]
    fn test_subnormal() {
        // экспонента=0, J=0, дробь≠0
        let sub = F80::from_parts(false, 0, 1);
        assert_eq!(sub.classify(), F80Class::Subnormal);
    }

    #[test]
    fn test_unnormal() {
        // экспонента=0, J=1, дробь≠0
        let un = F80::from_parts(false, 0, (1 << 63) | 1);
        assert_eq!(un.classify(), F80Class::Unnormal);
    }

    #[test]
    fn test_normal() {
        // экспонента в диапазоне, J=1
        let n = F80::from_parts(false, 16384, 1 << 63);
        assert_eq!(n.classify(), F80Class::Normal);
        assert!(n.is_finite());
    }

    #[test]
    fn test_denormal() {
        // экспонента≠0, но J=0
        let d = F80::from_parts(false, 100, 1);
        assert_eq!(d.classify(), F80Class::Denormal);
    }

    #[test]
    fn test_pseudo_infinity() {
        // экспонента=0x7FFF, J=0, дробь=0
        let pi = F80::from_parts(false, EXP_MAX, 0);
        assert_eq!(pi.classify(), F80Class::PseudoInfinity);
    }

    #[test]
    fn test_read_write_roundtrip() {
    let mut mem = Motherboard::new(64);
    let val = F80::from_parts(true, 0x4002, 0xC000_0000_0000_0000);
    val.write_to(&mut mem, 16);
    let loaded = F80::read_from(&mem, 16);
    assert_eq!(val, loaded);
}

#[test]
fn test_one_read_write() {
    let mut mem = Motherboard::new(64);
    let one = F80::one(false);
    one.write_to(&mut mem, 0);
    assert_eq!(mem.read_mem_u32(0), 0x0000_0000);
    assert_eq!(mem.read_mem_u32(4), 0x8000_0000);
    assert_eq!(mem.read_mem_u16(8), 0x3FFF);
}
}