// Ver: 1 File: crate/x86/src/fpu/convert.rs
//! Конверсии между F80 и целыми / двоичными float форматами.

use super::arith::{ExceptionFlags, RoundingMode};
use super::f80::{EXP_BIAS, EXP_MAX, F80, F80Class};

/// Нормализованное конечное число:
/// value = mant * 2^(exp - 63), где bit 63 в mant == 1.
#[derive(Clone, Copy, Debug)]
struct Norm {
    sign: bool,
    exp: i32,
    mant: u64,
}

/// Нормализует конечное F80.
/// Возвращает None для нуля, NaN и Infinity.
fn normalize(f: &F80) -> Option<Norm> {
    match f.classify() {
        F80Class::Zero
        | F80Class::Infinity
        | F80Class::QNaN
        | F80Class::SNaN
        | F80Class::PseudoInfinity
        | F80Class::PseudoNaN => None,
        _ => {
            let mant0 = f.mant;
            if mant0 == 0 {
                return None;
            }

            let biased = f.biased_exp() as i32;
            let raw_exp = if biased == 0 {
                1 - EXP_BIAS as i32
            } else {
                biased - EXP_BIAS as i32
            };

            let lz = mant0.leading_zeros() as i32;
            let mant = mant0 << lz;

            // value = mant0 * 2^(raw_exp - 63)
            //       = (mant0 << lz) * 2^(raw_exp - lz - 63)
            // Значит exp = raw_exp - lz.
            let exp = raw_exp - lz;

            Some(Norm {
                sign: f.sign(),
                exp,
                mant,
            })
        }
    }
}

/// Сдвиг вправо с возвратом guard/round/sticky.
/// Возвращает (quotient, guard, round, sticky, any_lost).
fn round_shift(x: u128, shift: u32) -> (u128, bool, bool, bool, bool) {
    if shift == 0 {
        return (x, false, false, false, false);
    }

    if shift >= 128 {
        return (0, false, false, x != 0, x != 0);
    }

    let q = x >> shift;
    let mask = if shift == 128 {
        u128::MAX
    } else {
        (1u128 << shift) - 1
    };
    let rem = x & mask;

    let guard = ((x >> (shift - 1)) & 1) != 0;
    let round = if shift >= 2 {
        ((x >> (shift - 2)) & 1) != 0
    } else {
        false
    };
    let sticky = if shift >= 3 {
        (x & ((1u128 << (shift - 2)) - 1)) != 0
    } else {
        false
    };

    (q, guard, round, sticky, rem != 0)
}

fn should_increment(
    q: u128,
    guard: bool,
    round: bool,
    sticky: bool,
    any_lost: bool,
    sign: bool,
    rm: RoundingMode,
) -> bool {
    match rm {
        RoundingMode::NearestEven => guard && (round || sticky || (q & 1) != 0),
        RoundingMode::Down => sign && any_lost,
        RoundingMode::Up => !sign && any_lost,
        RoundingMode::TowardZero => false,
    }
}

// ---------------------------------------------------------------------------
// Целые -> F80
// ---------------------------------------------------------------------------

fn from_u64_mag(sign: bool, mag: u64) -> F80 {
    if mag == 0 {
        return F80::zero(sign);
    }

    let lz = mag.leading_zeros() as i32;
    let mant = mag << lz;

    // value = mag = mant * 2^(-lz)
    // F80: value = mant * 2^(exp - 63)
    // exp - 63 = -lz => exp = 63 - lz
    let exp = 63 - lz;
    let biased = EXP_BIAS as i32 + exp;

    F80::from_parts(sign, biased as u16, mant)
}

pub fn from_i16(v: i16) -> F80 {
    let sign = v < 0;
    let mag = if sign {
        (v as i128).unsigned_abs() as u64
    } else {
        v as u64
    };
    from_u64_mag(sign, mag)
}

pub fn from_i32(v: i32) -> F80 {
    let sign = v < 0;
    let mag = if sign {
        (v as i128).unsigned_abs() as u64
    } else {
        v as u64
    };
    from_u64_mag(sign, mag)
}

pub fn from_i64(v: i64) -> F80 {
    let sign = v < 0;
    let mag = if sign {
        (v as u64).wrapping_neg()
    } else {
        v as u64
    };
    from_u64_mag(sign, mag)
}

// ---------------------------------------------------------------------------
// F80 -> целые
// ---------------------------------------------------------------------------

fn indefinite_int(bits: u32) -> i64 {
    // Integer indefinite: минимальное отрицательное значение данного размера.
    let min_val = 1i128 << (bits - 1);
    (-(min_val)) as i64
}

fn to_int_generic(f: F80, bits: u32, rm: RoundingMode) -> (i64, ExceptionFlags) {
    let mut flags = ExceptionFlags::default();

    if f.is_denormal() {
        flags.denormal = true;
    }

    match normalize(&f) {
        None => {
            // Ноль -> 0.
            if f.classify() == F80Class::Zero {
                return (0, flags);
            }

            // NaN / Infinity -> invalid.
            flags.invalid = true;
            (indefinite_int(bits), flags)
        }
        Some(n) => {
            let mag: u128;
            let inexact;

            if n.exp >= 63 {
                let shift = (n.exp - 63) as u32;
                if shift >= 128 {
                    flags.invalid = true;
                    return (indefinite_int(bits), flags);
                }
                mag = (n.mant as u128) << shift;
                inexact = false;
            } else {
                let shift = (63 - n.exp) as u32;
                let (q, guard, round, sticky, any_lost) = round_shift(n.mant as u128, shift);

                let inc = should_increment(q, guard, round, sticky, any_lost, n.sign, rm);
                mag = q + (inc as u128);
                inexact = any_lost;
            }

            if inexact {
                flags.precision = true;
            }

            let max_pos = (1u128 << (bits - 1)) - 1;
            let max_neg_mag = 1u128 << (bits - 1);

            let ok = if n.sign {
                mag <= max_neg_mag
            } else {
                mag <= max_pos
            };

            if !ok {
                flags.invalid = true;
                return (indefinite_int(bits), flags);
            }

            let signed = if n.sign { -(mag as i128) } else { mag as i128 };

            (signed as i64, flags)
        }
    }
}

pub fn to_i16(f: F80, rm: RoundingMode) -> (i16, ExceptionFlags) {
    let (v, flags) = to_int_generic(f, 16, rm);
    (v as i16, flags)
}

pub fn to_i32(f: F80, rm: RoundingMode) -> (i32, ExceptionFlags) {
    let (v, flags) = to_int_generic(f, 32, rm);
    (v as i32, flags)
}

pub fn to_i64(f: F80, rm: RoundingMode) -> (i64, ExceptionFlags) {
    to_int_generic(f, 64, rm)
}

// ---------------------------------------------------------------------------
// f32/f64 -> F80
// ---------------------------------------------------------------------------

/// value = sig * 2^exp2, sig != 0, sig помещается в u64.
fn from_sig_exp2(sign: bool, sig: u64, exp2: i32) -> F80 {
    if sig == 0 {
        return F80::zero(sign);
    }

    let lz = sig.leading_zeros() as i32;
    let mant = sig << lz;

    // value = sig * 2^exp2
    //       = mant * 2^(exp2 - lz)
    // F80: value = mant * 2^(exp - 63)
    // exp - 63 = exp2 - lz => exp = exp2 - lz + 63
    let exp = exp2 - lz + 63;
    let biased = EXP_BIAS as i32 + exp;

    F80::from_parts(sign, biased as u16, mant)
}

pub fn from_f32_bits(bits: u32) -> F80 {
    let sign = (bits >> 31) & 1 != 0;
    let exp = ((bits >> 23) & 0xFF) as i32;
    let frac = (bits & 0x007F_FFFF) as u64;

    if exp == 0 {
        if frac == 0 {
            return F80::zero(sign);
        }
        // Subnormal: value = frac * 2^-149
        return from_sig_exp2(sign, frac, -149);
    }

    if exp == 0xFF {
        if frac == 0 {
            return F80::infinity(sign);
        }

        // NaN.
        let quiet = (frac >> 22) & 1 != 0;
        let payload = frac & 0x003F_FFFF;
        let f80_frac = if quiet { 1u64 << 62 } else { 0 } | ((payload as u64) << 40);
        return F80::from_parts(sign, EXP_MAX, (1u64 << 63) | f80_frac);
    }

    // Normal: value = (2^23 + frac) * 2^(exp - 127 - 23)
    let sig = (1u64 << 23) | frac;
    let exp2 = exp - 127 - 23;
    from_sig_exp2(sign, sig, exp2)
}

pub fn from_f64_bits(bits: u64) -> F80 {
    let sign = (bits >> 63) & 1 != 0;
    let exp = ((bits >> 52) & 0x7FF) as i32;
    let frac = bits & 0x000F_FFFF_FFFF_FFFF;

    if exp == 0 {
        if frac == 0 {
            return F80::zero(sign);
        }
        // Subnormal: value = frac * 2^-1074
        return from_sig_exp2(sign, frac, -1074);
    }

    if exp == 0x7FF {
        if frac == 0 {
            return F80::infinity(sign);
        }

        // NaN.
        let quiet = (frac >> 51) & 1 != 0;
        let payload = frac & 0x0007_FFFF_FFFF_FFFF;
        let f80_frac = if quiet { 1u64 << 62 } else { 0 } | (payload << 11);
        return F80::from_parts(sign, EXP_MAX, (1u64 << 63) | f80_frac);
    }

    // Normal: value = (2^52 + frac) * 2^(exp - 1023 - 52)
    let sig = (1u64 << 52) | frac;
    let exp2 = exp - 1023 - 52;
    from_sig_exp2(sign, sig, exp2)
}

#[inline]
pub fn from_f32(v: f32) -> F80 {
    from_f32_bits(v.to_bits())
}

#[inline]
pub fn from_f64(v: f64) -> F80 {
    from_f64_bits(v.to_bits())
}

// ---------------------------------------------------------------------------
// F80 -> f32/f64
// ---------------------------------------------------------------------------

fn overflow_to_binary(
    sign: bool,
    exp_bits: u32,
    frac_bits: u32,
    rm: RoundingMode,
    flags: &mut ExceptionFlags,
) -> u64 {
    flags.overflow = true;
    flags.precision = true;

    let total_bits = exp_bits + frac_bits + 1;
    let sign_shift = total_bits - 1;
    let max_biased = (1u32 << exp_bits) - 2;
    let inf_biased = (1u32 << exp_bits) - 1;
    let frac_mask = (1u128 << frac_bits) - 1;

    let to_inf = match rm {
        RoundingMode::NearestEven => true,
        RoundingMode::TowardZero => false,
        RoundingMode::Up => !sign,
        RoundingMode::Down => sign,
    };

    let exp_field = if to_inf { inf_biased } else { max_biased };
    let frac = if to_inf { 0 } else { frac_mask as u64 };

    ((sign as u64) << sign_shift) | ((exp_field as u64) << frac_bits) | frac
}

fn nan_to_binary(f: &F80, exp_bits: u32, frac_bits: u32, flags: &mut ExceptionFlags) -> u64 {
    let sign = f.sign();
    let cls = f.classify();

    if cls == F80Class::SNaN || cls == F80Class::PseudoNaN {
        flags.invalid = true;
    }

    let total_bits = exp_bits + frac_bits + 1;
    let sign_shift = total_bits - 1;
    let inf_biased = (1u32 << exp_bits) - 1;
    let quiet_bit = 1u64 << (frac_bits - 1);
    let payload_bits = frac_bits - 1;

    // F80 fraction: bit 62 = quiet, bits 61..0 = payload.
    let src_payload = f.fraction();
    let shift = 62 - payload_bits;
    let mut payload = (src_payload >> shift) & ((1u64 << payload_bits) - 1);
    if payload == 0 {
        payload = 1;
    }

    ((sign as u64) << sign_shift) | ((inf_biased as u64) << frac_bits) | quiet_bit | payload
}

fn to_binary_generic(
    f: F80,
    exp_bits: u32,
    frac_bits: u32,
    bias: i32,
    rm: RoundingMode,
) -> (u64, ExceptionFlags) {
    let mut flags = ExceptionFlags::default();

    if f.is_denormal() {
        flags.denormal = true;
    }

    let total_bits = exp_bits + frac_bits + 1;
    let sign_shift = total_bits - 1;
    let max_biased = (1u32 << exp_bits) - 2;
    let frac_mask = (1u128 << frac_bits) - 1;
    let min_normal_sig = 1u128 << frac_bits;
    let max_normal_sig = 1u128 << (frac_bits + 1);

    match f.classify() {
        F80Class::Zero => {
            return (((f.sign() as u64) << sign_shift), flags);
        }
        F80Class::Infinity | F80Class::PseudoInfinity => {
            let exp_field = max_biased + 1;
            return (
                ((f.sign() as u64) << sign_shift) | ((exp_field as u64) << frac_bits),
                flags,
            );
        }
        F80Class::QNaN | F80Class::SNaN | F80Class::PseudoNaN => {
            let bits = nan_to_binary(&f, exp_bits, frac_bits, &mut flags);
            return (bits, flags);
        }
        _ => {}
    }

    let n = match normalize(&f) {
        Some(n) => n,
        None => {
            // Должно быть недостижимо, но на всякий случай вернём ноль.
            return (((f.sign() as u64) << sign_shift), flags);
        }
    };

    let emin = 1 - bias;
    let emax = max_biased as i32 - bias;

    if n.exp > emax {
        let bits = overflow_to_binary(n.sign, exp_bits, frac_bits, rm, &mut flags);
        return (bits, flags);
    }

    let normal_shift = 63 - frac_bits; // 64 - (frac_bits + 1)

    if n.exp < emin {
        // Subnormal target.
        let extra = (emin - n.exp) as u32;
        let shift = normal_shift as u32 + extra;

        let (q, guard, round, sticky, any_lost) = round_shift(n.mant as u128, shift);
        let inc = should_increment(q, guard, round, sticky, any_lost, n.sign, rm);
        let mut sig = q + (inc as u128);
        let inexact = any_lost;

        if sig >= min_normal_sig {
            // Округление подняло до минимального нормального.
            if inexact {
                flags.precision = true;
            }
            let exp_field = 1u64;
            let frac = 0u64;
            let bits = ((n.sign as u64) << sign_shift) | (exp_field << frac_bits) | frac;
            return (bits, flags);
        }

        if inexact {
            flags.underflow = true;
            flags.precision = true;
        }

        let exp_field = 0u64;
        let frac = (sig & frac_mask) as u64;
        let bits = ((n.sign as u64) << sign_shift) | (exp_field << frac_bits) | frac;
        return (bits, flags);
    }

    // Normal target.
    let shift = normal_shift as u32;
    let (q, guard, round, sticky, any_lost) = round_shift(n.mant as u128, shift);
    let inc = should_increment(q, guard, round, sticky, any_lost, n.sign, rm);
    let mut sig = q + (inc as u128);
    let inexact = any_lost;

    let mut exp = n.exp;

    if sig >= max_normal_sig {
        sig = min_normal_sig;
        exp += 1;

        if exp > emax {
            let bits = overflow_to_binary(n.sign, exp_bits, frac_bits, rm, &mut flags);
            return (bits, flags);
        }
    }

    if inexact {
        flags.precision = true;
    }

    let exp_field = (exp + bias) as u64;
    let frac = (sig & frac_mask) as u64;

    let bits = ((n.sign as u64) << sign_shift) | (exp_field << frac_bits) | frac;

    (bits, flags)
}

pub fn to_f64_bits(f: F80, rm: RoundingMode) -> (u64, ExceptionFlags) {
    to_binary_generic(f, 11, 52, 1023, rm)
}

pub fn to_f32_bits(f: F80, rm: RoundingMode) -> (u32, ExceptionFlags) {
    let (bits, flags) = to_binary_generic(f, 8, 23, 127, rm);
    (bits as u32, flags)
}

#[inline]
pub fn to_f64(f: F80, rm: RoundingMode) -> (f64, ExceptionFlags) {
    let (bits, flags) = to_f64_bits(f, rm);
    (f64::from_bits(bits), flags)
}

#[inline]
pub fn to_f32(f: F80, rm: RoundingMode) -> (f32, ExceptionFlags) {
    let (bits, flags) = to_f32_bits(f, rm);
    (f32::from_bits(bits), flags)
}

// ---------------------------------------------------------------------------
// Packed BCD (FBLD / FBSTP)
// ---------------------------------------------------------------------------

/// Количество десятичных цифр в 18-разрядном packed BCD.
pub const BCD_DIGITS: usize = 18;

/// 10^18 — граница диапазона packed BCD.
const BCD_LIMIT: u128 = 1_000_000_000_000_000_000;

/// Все 1s — значение приёмника FBSTP при ошибке/переполнении.
const BCD_ALL_ONES: [u8; 10] = [0xFF; 10];

/// FBLD m80: 10-байтный packed BCD -> F80.
///
/// Формат (little-endian, 18 десятичных цифр):
/// - байты 0..7: по две цифры, младшая цифра в младшем ниббле;
/// - байт 8: цифра 17 в младшем ниббле, цифра 18 в старшем;
/// - байт 9: бит 7 — знак (1 = отрицательное), биты 6..0 игнорируются.
///
/// Любая цифра > 9 -> invalid operation (#IA).
/// Результат всегда точный: 10^18 - 1 < 2^64, мантисса F80 — 64 бита.
pub fn from_bcd(bytes: &[u8; 10]) -> (F80, ExceptionFlags) {
    let mut flags = ExceptionFlags::default();
    let sign = bytes[9] & 0x80 != 0;

    let mut mag: u128 = 0;
    let mut place: u128 = 1;
    let mut invalid = false;

    for i in 0..9usize {
        let b = bytes[i];
        for &nibble in &[b & 0x0F, (b >> 4) & 0x0F] {
            if nibble > 9 {
                invalid = true;
            }
            mag += (nibble as u128) * place;
            place *= 10;
        }
    }

    if invalid {
        // Спецификация: #IA, значение в приёмнике не определено.
        // Декодируем «как есть» (нибблы 10..15 считаются своими весами),
        // чтобы поведение было детерминированным.
        flags.invalid = true;
        log::warn!("FBLD: invalid packed BCD digit, bytes={:02X?}", bytes);
    }

    if mag == 0 {
        return (F80::zero(sign), flags);
    }

    // mag < 10^18 для валидных данных; даже с нибблами до 15
    // максимум ~1.67e18 < 2^64, так что сужение безопасно.
    (from_u64_mag(sign, mag as u64), flags)
}

/// FBSTP m80: F80 -> 10-байтный packed BCD.
///
/// Всегда уссекает к нулю, независимо от режима округления.
/// NaN / Infinity / |value| >= 10^18 -> #IA и приёмник из всех 1s.
pub fn to_bcd(f: F80) -> ([u8; 10], ExceptionFlags) {
    let mut flags = ExceptionFlags::default();

    if f.is_denormal() {
        flags.denormal = true;
    }

    match f.classify() {
        F80Class::Zero => {
            let mut out = [0u8; 10];
            if f.sign() {
                out[9] = 0x80;
            }
            return (out, flags);
        }
        F80Class::Infinity
        | F80Class::PseudoInfinity
        | F80Class::QNaN
        | F80Class::SNaN
        | F80Class::PseudoNaN => {
            flags.invalid = true;
            return (BCD_ALL_ONES, flags);
        }
        _ => {}
    }

    let n = match normalize(&f) {
        Some(n) => n,
        None => {
            flags.invalid = true;
            return (BCD_ALL_ONES, flags);
        }
    };

    // Целая часть магнитуды, усечение к нулю.
    let mag: u128 = if n.exp >= 63 {
        let shift = (n.exp - 63) as u32;
        if shift >= 64 {
            // >= 2^64 > 10^18 — заведомо переполнение.
            flags.invalid = true;
            return (BCD_ALL_ONES, flags);
        }
        (n.mant as u128) << shift
    } else {
        let shift = (63 - n.exp) as u32;
        if shift >= 64 {
            0
        } else {
            (n.mant >> shift) as u128
        }
    };

    if mag >= BCD_LIMIT {
        flags.invalid = true;
        return (BCD_ALL_ONES, flags);
    }

    let mut out = [0u8; 10];
    let mut v = mag;
    for i in 0..9usize {
        let d0 = (v % 10) as u8;
        v /= 10;
        let d1 = (v % 10) as u8;
        v /= 10;
        out[i] = (d1 << 4) | d0;
    }
    debug_assert_eq!(v, 0, "18 digits must consume the whole magnitude");

    if n.sign {
        out[9] = 0x80;
    }

    (out, flags)
}

#[cfg(test)]
mod tests {
    use crate::fpu::{arith::{cmp, mul}, neg};

    use super::*;
    use super::super::arith::CmpResult;

    const F32_ONE: u32 = 0x3F80_0000;
    const F32_HALF: u32 = 0x3F00_0000;
    const F32_1_5: u32 = 0x3FC0_0000;
    const F32_NEG_1_5: u32 = 0xBFC0_0000;
    const F64_ONE: u64 = 0x3FF0_0000_0000_0000;

    #[test]
    fn test_from_i32_zero() {
        let f = from_i32(0);
        assert_eq!(f.classify(), F80Class::Zero);
    }

    #[test]
    fn test_from_i32_one() {
        let f = from_i32(1);
        assert_eq!(f, F80::one(false));
    }

    #[test]
    fn test_from_i32_minus_one() {
        let f = from_i32(-1);
        assert_eq!(f, F80::one(true));
    }

    #[test]
    fn test_from_i32_roundtrip() {
        for v in [-1000i32, -1, 0, 1, 12345, i32::MIN, i32::MAX] {
            let f = from_i32(v);
            let (back, flags) = to_i32(f, RoundingMode::TowardZero);
            assert_eq!(back, v, "roundtrip failed for {}", v);
            assert!(!flags.invalid);
        }
    }

    #[test]
    fn test_from_i64_min_roundtrip() {
        let v = i64::MIN;
        let f = from_i64(v);
        let (back, flags) = to_i64(f, RoundingMode::TowardZero);
        assert_eq!(back, v);
        assert!(!flags.invalid);
    }

    #[test]
    fn test_to_i32_overflow() {
        let f = from_i64(1i64 << 31);
        let (v, flags) = to_i32(f, RoundingMode::TowardZero);
        assert!(flags.invalid);
        assert_eq!(v, i32::MIN);
    }

    #[test]
    fn test_round_1_5_nearest_even() {
        let f = from_f32_bits(F32_1_5);
        let (v, _) = to_i32(f, RoundingMode::NearestEven);
        assert_eq!(v, 2);
    }

    #[test]
    fn test_round_1_5_toward_zero() {
        let f = from_f32_bits(F32_1_5);
        let (v, _) = to_i32(f, RoundingMode::TowardZero);
        assert_eq!(v, 1);
    }

    #[test]
    fn test_round_1_5_up() {
        let f = from_f32_bits(F32_1_5);
        let (v, _) = to_i32(f, RoundingMode::Up);
        assert_eq!(v, 2);
    }

    #[test]
    fn test_round_1_5_down() {
        let f = from_f32_bits(F32_1_5);
        let (v, _) = to_i32(f, RoundingMode::Down);
        assert_eq!(v, 1);
    }

    #[test]
    fn test_round_neg_1_5_nearest_even() {
        let f = from_f32_bits(F32_NEG_1_5);
        let (v, _) = to_i32(f, RoundingMode::NearestEven);
        assert_eq!(v, -2);
    }

    #[test]
    fn test_round_neg_1_5_toward_zero() {
        let f = from_f32_bits(F32_NEG_1_5);
        let (v, _) = to_i32(f, RoundingMode::TowardZero);
        assert_eq!(v, -1);
    }

    #[test]
    fn test_round_neg_1_5_up() {
        let f = from_f32_bits(F32_NEG_1_5);
        let (v, _) = to_i32(f, RoundingMode::Up);
        assert_eq!(v, -1);
    }

    #[test]
    fn test_round_neg_1_5_down() {
        let f = from_f32_bits(F32_NEG_1_5);
        let (v, _) = to_i32(f, RoundingMode::Down);
        assert_eq!(v, -2);
    }

    #[test]
    fn test_from_f32_one() {
        let f = from_f32_bits(F32_ONE);
        assert_eq!(f, F80::one(false));
    }

    #[test]
    fn test_from_f32_half() {
        let f = from_f32_bits(F32_HALF);
        // 0.5 = 2^-1 => exp field = -1, biased = 16382, mant = 2^63
        let expected = F80::from_parts(false, EXP_BIAS as u16 - 1, 1u64 << 63);
        assert_eq!(f, expected);
    }

    #[test]
    fn test_to_f64_one() {
        let f = F80::one(false);
        let (bits, flags) = to_f64_bits(f, RoundingMode::NearestEven);
        assert_eq!(bits, F64_ONE);
        assert!(!flags.invalid);
        assert!(!flags.precision);
    }

    #[test]
    fn test_to_f32_one() {
        let f = F80::one(false);
        let (bits, flags) = to_f32_bits(f, RoundingMode::NearestEven);
        assert_eq!(bits, F32_ONE);
        assert!(!flags.invalid);
        assert!(!flags.precision);
    }

    #[test]
    fn test_f32_roundtrip_one() {
        let f = from_f32_bits(F32_ONE);
        let (bits, _) = to_f32_bits(f, RoundingMode::NearestEven);
        assert_eq!(bits, F32_ONE);
    }

    #[test]
    fn test_f32_roundtrip_half() {
        let f = from_f32_bits(F32_HALF);
        let (bits, _) = to_f32_bits(f, RoundingMode::NearestEven);
        assert_eq!(bits, F32_HALF);
    }

    #[test]
    fn test_f32_roundtrip_1_5() {
        let f = from_f32_bits(F32_1_5);
        let (bits, _) = to_f32_bits(f, RoundingMode::NearestEven);
        assert_eq!(bits, F32_1_5);
    }

    #[test]
    fn test_f64_roundtrip_one() {
        let f = from_f64_bits(F64_ONE);
        let (bits, _) = to_f64_bits(f, RoundingMode::NearestEven);
        assert_eq!(bits, F64_ONE);
    }

    #[test]
    fn test_to_f32_overflow() {
        // Очень большое число: exp = 200, mant = 2^63.
        let f = F80::from_parts(false, EXP_BIAS as u16 + 200, 1u64 << 63);
        let (_, flags) = to_f32_bits(f, RoundingMode::NearestEven);
        assert!(flags.overflow);
    }

    #[test]
    fn test_to_f64_underflow() {
        // Очень маленькое число: exp = -2000, mant = 2^63.
        let f = F80::from_parts(false, EXP_BIAS as u16 - 2000, 1u64 << 63);
        let (_, flags) = to_f64_bits(f, RoundingMode::NearestEven);
        assert!(flags.underflow);
    }

    const BCD_ZERO: [u8; 10] = [0; 10];
    const BCD_ONE: [u8; 10] = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    const BCD_NEG_ONE: [u8; 10] = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0x80];
    const BCD_123456789012345678: [u8; 10] =
        [0x78, 0x56, 0x34, 0x12, 0x90, 0x78, 0x56, 0x34, 0x12, 0x00];

    #[test]
    fn test_from_bcd_zero() {
        let (f, flags) = from_bcd(&BCD_ZERO);
        assert_eq!(f.classify(), F80Class::Zero);
        assert!(!f.sign());
        assert!(!flags.invalid);
    }

    #[test]
    fn test_from_bcd_negative_zero() {
        let mut b = BCD_ZERO;
        b[9] = 0x80;
        let (f, _) = from_bcd(&b);
        assert_eq!(f.classify(), F80Class::Zero);
        assert!(f.sign());
    }

    #[test]
    fn test_from_bcd_one() {
        let (f, flags) = from_bcd(&BCD_ONE);
        assert_eq!(f, F80::one(false));
        assert!(!flags.invalid);
    }

    #[test]
    fn test_from_bcd_neg_one() {
        let (f, _) = from_bcd(&BCD_NEG_ONE);
        assert_eq!(f, F80::one(true));
    }

    #[test]
    fn test_from_bcd_18_digits_exact() {
        let (f, flags) = from_bcd(&BCD_123456789012345678);
        assert!(!flags.invalid);
        assert!(!flags.precision, "BCD must convert exactly");
        let (back, f2) = to_i64(f, RoundingMode::TowardZero);
        assert!(!f2.invalid);
        assert_eq!(back, 123_456_789_012_345_678i64);
    }

    #[test]
    fn test_from_bcd_ignores_low_bits_of_sign_byte() {
        // Биты 6..0 байта 9 игнорируются при загрузке.
        let mut b = BCD_ONE;
        b[9] = 0x7F; // без знака, но мусор в младших битах
        let (f, flags) = from_bcd(&b);
        assert_eq!(f, F80::one(false));
        assert!(!flags.invalid);
    }

    #[test]
    fn test_from_bcd_invalid_digit() {
        let mut b = BCD_ONE;
        b[0] = 0x0A; // цифра 10 — невалидна
        let (_, flags) = from_bcd(&b);
        assert!(flags.invalid);
    }

    #[test]
    fn test_to_bcd_zero() {
        let (out, flags) = to_bcd(F80::zero(false));
        assert_eq!(out, BCD_ZERO);
        assert!(!flags.invalid);
    }

    #[test]
    fn test_to_bcd_negative_zero_keeps_sign() {
        let (out, _) = to_bcd(F80::zero(true));
        assert_eq!(out[9], 0x80);
        assert_eq!(out[0], 0);
    }

    #[test]
    fn test_to_bcd_one() {
        let (out, flags) = to_bcd(F80::one(false));
        assert_eq!(out, BCD_ONE);
        assert!(!flags.invalid);
    }

    #[test]
    fn test_to_bcd_neg_one() {
        let (out, _) = to_bcd(F80::one(true));
        assert_eq!(out, BCD_NEG_ONE);
    }

    #[test]
    fn test_to_bcd_truncates_toward_zero() {
        // 123.999 -> 123, независимо от rounding mode.
        let f = from_f64(123.999);
        for rm in [
            RoundingMode::NearestEven,
            RoundingMode::Down,
            RoundingMode::Up,
            RoundingMode::TowardZero,
        ] {
            let (out, _) = to_bcd(f);
            assert_eq!(out[0], 0x23, "rm={:?}", rm);
            assert_eq!(out[1], 0x01, "rm={:?}", rm);
            assert_eq!(&out[2..], &[0u8; 8][..], "rm={:?}", rm);
        }
    }

    #[test]
    fn test_to_bcd_truncates_negative() {
        let f = neg(from_f64(123.999));
        let (out, _) = to_bcd(f);
        assert_eq!(out[0], 0x23);
        assert_eq!(out[1], 0x01);
        assert_eq!(out[9], 0x80);
    }

    #[test]
    fn test_to_bcd_overflow() {
        // Ровно 10^18 — уже не влезает в 18 цифр.
        let f = from_i64(i64::MAX);
        let big = mul(f, from_i64(2), RoundingMode::TowardZero).0;

        // big = 2*(2^63 - 1) = 2^64 - 2 > 123456789012345678,
        // значит точно за границей 18-разрядного BCD.
        let (order, _) = cmp(big, from_bcd(&BCD_123456789012345678).0);
        assert_eq!(order, CmpResult::Greater);

        let (out, flags) = to_bcd(big);
        assert!(flags.invalid);
        assert_eq!(out, BCD_ALL_ONES);
    }

    #[test]
    fn test_to_bcd_max_valid() {
        // 10^18 - 1 — максимально допустимое.
        let (f, flags) = from_bcd(&[0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x00]);
        assert!(!flags.invalid);
        let (out, flags2) = to_bcd(f);
        assert!(!flags2.invalid);
        assert_eq!(out[0], 0x99);
        assert_eq!(out[8], 0x99);
        assert_eq!(out[9], 0x00);
    }

    #[test]
    fn test_to_bcd_nan() {
        let (out, flags) = to_bcd(F80::qnan());
        assert!(flags.invalid);
        assert_eq!(out, BCD_ALL_ONES);
    }

    #[test]
    fn test_to_bcd_inf() {
        let (out, flags) = to_bcd(F80::infinity(false));
        assert!(flags.invalid);
        assert_eq!(out, BCD_ALL_ONES);
    }

    #[test]
    fn test_bcd_roundtrip() {
        let (f, flags) = from_bcd(&BCD_123456789012345678);
        assert!(!flags.invalid);
        let (out, flags2) = to_bcd(f);
        assert!(!flags2.invalid);
        assert_eq!(out, BCD_123456789012345678);
    }
}
