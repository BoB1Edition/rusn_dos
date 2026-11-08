// Ver: 4 File: crate/x86/src/fpu/arith.rs

use super::f80::{EXP_BIAS, EXP_MAX, F80, F80Class};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundingMode {
    NearestEven,
    Down,
    Up,
    TowardZero,
}

impl RoundingMode {
    pub fn from_control(cw: u16) -> Self {
        match (cw >> 10) & 0x3 {
            0 => RoundingMode::NearestEven,
            1 => RoundingMode::Down,
            2 => RoundingMode::Up,
            _ => RoundingMode::TowardZero,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExceptionFlags {
    pub invalid: bool,
    pub denormal: bool,
    pub div_zero: bool,
    pub overflow: bool,
    pub underflow: bool,
    pub precision: bool,
}

const MIN_NORMAL_EXP: i32 = 1 - EXP_BIAS as i32;
const MAX_NORMAL_EXP: i32 = (EXP_MAX as i32 - 1) - EXP_BIAS as i32;

#[derive(Clone, Copy, Debug)]
struct Unpacked {
    sign: bool,
    exp: i32,
    mant: u64,
    kind: UnpackedKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnpackedKind {
    Zero,
    Normal,
    Inf,
    QNaN,
    SNaN,
}

fn unpack(f: &F80) -> Unpacked {
    let sign = f.sign();
    let biased_exp = f.biased_exp();
    let mant = f.mant;

    match f.classify() {
        F80Class::Zero => Unpacked {
            sign,
            exp: 0,
            mant: 0,
            kind: UnpackedKind::Zero,
        },
        F80Class::Infinity | F80Class::PseudoInfinity => Unpacked {
            sign,
            exp: 0,
            mant: 0,
            kind: UnpackedKind::Inf,
        },
        F80Class::QNaN | F80Class::PseudoNaN => Unpacked {
            sign,
            exp: 0,
            mant,
            kind: UnpackedKind::QNaN,
        },
        F80Class::SNaN => Unpacked {
            sign,
            exp: 0,
            mant,
            kind: UnpackedKind::SNaN,
        },
        _ => {
            if mant == 0 {
                return Unpacked {
                    sign,
                    exp: 0,
                    mant: 0,
                    kind: UnpackedKind::Zero,
                };
            }
            let lz = mant.leading_zeros() as i32;
            let norm_mant = mant << lz;
            let base_exp = if biased_exp == 0 {
                1 - EXP_BIAS as i32
            } else {
                biased_exp as i32 - EXP_BIAS as i32
            };
            let exp = base_exp - lz;
            Unpacked {
                sign,
                exp,
                mant: norm_mant,
                kind: UnpackedKind::Normal,
            }
        }
    }
}

fn shift_right_sticky(sig: u128, shift: u32) -> (u128, bool) {
    if shift == 0 {
        return (sig, false);
    }
    if shift >= 128 {
        return (0, sig != 0);
    }
    let lost = sig & ((1u128 << shift) - 1);
    (sig >> shift, lost != 0)
}

fn shift_right_grs(sig: u128, n: u32) -> (u128, bool, bool, bool) {
    if n == 0 {
        return (sig, false, false, false);
    }
    if n >= 128 {
        return (0, false, false, sig != 0);
    }
    let result = sig >> n;
    let guard = ((sig >> (n - 1)) & 1) != 0;
    let round = if n >= 2 {
        ((sig >> (n - 2)) & 1) != 0
    } else {
        false
    };
    let sticky = if n >= 3 {
        (sig & ((1u128 << (n - 2)) - 1)) != 0
    } else {
        false
    };
    (result, guard, round, sticky)
}

fn round_pack(
    sign: bool,
    mut exp: i32,
    mut sig: u128,
    sticky: bool,
    rm: RoundingMode,
    flags: &mut ExceptionFlags,
) -> F80 {
    if sig == 0 {
        let zero_sign = rm == RoundingMode::Down && sign;
        return F80::zero(zero_sign);
    }

    let lz = sig.leading_zeros() as i32;
    sig <<= lz;
    exp -= lz;

    let mut f80_exp = exp + 1;

    let mut sig64 = (sig >> 64) as u64;
    let guard = ((sig >> 63) & 1) != 0;
    let round_bit = ((sig >> 62) & 1) != 0;
    let sticky_bits = (sig & ((1u128 << 62) - 1)) != 0;
    let any_lost = guard || round_bit || sticky_bits || sticky;

    let increment = match rm {
        RoundingMode::NearestEven => {
            guard && (round_bit || sticky_bits || sticky || (sig64 & 1 != 0))
        }
        RoundingMode::Down => sign && any_lost,
        RoundingMode::Up => !sign && any_lost,
        RoundingMode::TowardZero => false,
    };

    if increment {
        if sig64 == u64::MAX {
            sig64 = 1 << 63;
            f80_exp += 1;
        } else {
            sig64 += 1;
        }
    }
    if any_lost {
        flags.precision = true;
    }

    if f80_exp > MAX_NORMAL_EXP {
        flags.overflow = true;
        flags.precision = true;
        let to_inf = match rm {
            RoundingMode::NearestEven => true,
            RoundingMode::Down => sign,
            RoundingMode::Up => !sign,
            RoundingMode::TowardZero => false,
        };
        return if to_inf {
            F80::infinity(sign)
        } else {
            F80::from_parts(sign, EXP_MAX - 1, u64::MAX)
        };
    }

    if f80_exp < MIN_NORMAL_EXP {
        let shift = (MIN_NORMAL_EXP - f80_exp) as u32 + 64;
        let (shifted, d_guard, d_round, d_sticky_bits) = shift_right_grs(sig, shift);
        let mut denorm_mant = shifted as u64;
        let d_any_lost = d_guard || d_round || d_sticky_bits || sticky;

        let d_increment = match rm {
            RoundingMode::NearestEven => {
                d_guard && (d_round || d_sticky_bits || sticky || (denorm_mant & 1 != 0))
            }
            RoundingMode::Down => sign && d_any_lost,
            RoundingMode::Up => !sign && d_any_lost,
            RoundingMode::TowardZero => false,
        };

        if d_increment {
            denorm_mant = denorm_mant.wrapping_add(1);
        }
        if d_any_lost {
            flags.precision = true;
            flags.underflow = true;
        }

        return if denorm_mant & (1 << 63) != 0 {
            F80::from_parts(sign, 1, denorm_mant)
        } else {
            F80::from_parts(sign, 0, denorm_mant)
        };
    }

    let biased = (f80_exp + EXP_BIAS as i32) as u16;
    F80::from_parts(sign, biased, sig64)
}

fn round_single(u: &Unpacked, rm: RoundingMode, flags: &mut ExceptionFlags) -> F80 {
    if u.kind == UnpackedKind::Zero {
        return F80::zero(u.sign);
    }
    let sig = (u.mant as u128) << 63;
    round_pack(u.sign, u.exp, sig, false, rm, flags)
}

pub fn add(a: F80, b: F80, rm: RoundingMode) -> (F80, ExceptionFlags) {
    let mut flags = ExceptionFlags::default();
    let ua = unpack(&a);
    let ub = unpack(&b);

    let a_nan = matches!(ua.kind, UnpackedKind::QNaN | UnpackedKind::SNaN);
    let b_nan = matches!(ub.kind, UnpackedKind::QNaN | UnpackedKind::SNaN);
    if a_nan || b_nan {
        if ua.kind == UnpackedKind::SNaN || ub.kind == UnpackedKind::SNaN {
            flags.invalid = true;
        }
        return (F80::qnan(), flags);
    }

    match (ua.kind, ub.kind) {
        (UnpackedKind::Inf, UnpackedKind::Inf) => {
            if ua.sign != ub.sign {
                flags.invalid = true;
                return (F80::qnan(), flags);
            }
            return (F80::infinity(ua.sign), flags);
        }
        (UnpackedKind::Inf, _) => return (F80::infinity(ua.sign), flags),
        (_, UnpackedKind::Inf) => return (F80::infinity(ub.sign), flags),
        _ => {}
    }

    match (ua.kind, ub.kind) {
        (UnpackedKind::Zero, UnpackedKind::Zero) => {
            let sign = if rm == RoundingMode::Down {
                ua.sign || ub.sign
            } else {
                ua.sign && ub.sign
            };
            return (F80::zero(sign), flags);
        }
        (UnpackedKind::Zero, _) => {
            if b.is_denormal() {
                flags.denormal = true;
            }
            return (round_single(&ub, rm, &mut flags), flags);
        }
        (_, UnpackedKind::Zero) => {
            if a.is_denormal() {
                flags.denormal = true;
            }
            return (round_single(&ua, rm, &mut flags), flags);
        }
        _ => {}
    }

    if a.is_denormal() || b.is_denormal() {
        flags.denormal = true;
    }

    let mut a_sig = (ua.mant as u128) << 63;
    let mut b_sig = (ub.mant as u128) << 63;
    let mut a_exp = ua.exp;
    let mut b_exp = ub.exp;
    let mut a_sign = ua.sign;
    let mut b_sign = ub.sign;

    if a_exp < b_exp {
        std::mem::swap(&mut a_sig, &mut b_sig);
        std::mem::swap(&mut a_exp, &mut b_exp);
        std::mem::swap(&mut a_sign, &mut b_sign);
    }
    let d = (a_exp - b_exp) as u32;
    let (shifted, mut sticky) = shift_right_sticky(b_sig, d);
    b_sig = shifted;

    let (result_sign, result_sig) = if a_sign == b_sign {
        (a_sign, a_sig + b_sig)
    } else if a_sig >= b_sig {
        (a_sign, a_sig - b_sig)
    } else {
        (b_sign, b_sig - a_sig)
    };

    if result_sig == 0 {
        let zero_sign = rm == RoundingMode::Down;
        return (F80::zero(zero_sign), flags);
    }

    let result = round_pack(result_sign, a_exp, result_sig, sticky, rm, &mut flags);
    (result, flags)
}

pub fn neg(f: F80) -> F80 {
    F80::from_parts(!f.sign(), f.biased_exp(), f.mant)
}

pub fn sub(a: F80, b: F80, rm: RoundingMode) -> (F80, ExceptionFlags) {
    add(a, neg(b), rm)
}

pub fn mul(a: F80, b: F80, rm: RoundingMode) -> (F80, ExceptionFlags) {
    let mut flags = ExceptionFlags::default();
    let ua = unpack(&a);
    let ub = unpack(&b);

    let a_nan = matches!(ua.kind, UnpackedKind::QNaN | UnpackedKind::SNaN);
    let b_nan = matches!(ub.kind, UnpackedKind::QNaN | UnpackedKind::SNaN);
    if a_nan || b_nan {
        if ua.kind == UnpackedKind::SNaN || ub.kind == UnpackedKind::SNaN {
            flags.invalid = true;
        }
        return (F80::qnan(), flags);
    }

    match (ua.kind, ub.kind) {
        (UnpackedKind::Inf, UnpackedKind::Zero) | (UnpackedKind::Zero, UnpackedKind::Inf) => {
            flags.invalid = true;
            return (F80::qnan(), flags);
        }
        (UnpackedKind::Inf, _) | (_, UnpackedKind::Inf) => {
            return (F80::infinity(ua.sign ^ ub.sign), flags);
        }
        (UnpackedKind::Zero, _) | (_, UnpackedKind::Zero) => {
            return (F80::zero(ua.sign ^ ub.sign), flags);
        }
        _ => {}
    }

    if a.is_denormal() || b.is_denormal() {
        flags.denormal = true;
    }

    let product = (ua.mant as u128) * (ub.mant as u128);

    let exp = ua.exp + ub.exp;

    let result = round_pack(ua.sign ^ ub.sign, exp, product, false, rm, &mut flags);
    (result, flags)
}

fn first_frac_bit(r: u128, d: u128) -> (bool, u128) {
    let twice = r << 1;
    if twice >= d {
        (true, twice - d)
    } else {
        (false, twice)
    }
}

pub fn div(a: F80, b: F80, rm: RoundingMode) -> (F80, ExceptionFlags) {
    let mut flags = ExceptionFlags::default();
    let ua = unpack(&a);
    let ub = unpack(&b);

    let a_nan = matches!(ua.kind, UnpackedKind::QNaN | UnpackedKind::SNaN);
    let b_nan = matches!(ub.kind, UnpackedKind::QNaN | UnpackedKind::SNaN);
    if a_nan || b_nan {
        if ua.kind == UnpackedKind::SNaN || ub.kind == UnpackedKind::SNaN {
            flags.invalid = true;
        }
        return (F80::qnan(), flags);
    }

    let sign = ua.sign ^ ub.sign;

    if ua.kind == UnpackedKind::Inf && ub.kind == UnpackedKind::Inf {
        flags.invalid = true;
        return (F80::qnan(), flags);
    }
    if ua.kind == UnpackedKind::Inf {
        return (F80::infinity(sign), flags);
    }
    if ub.kind == UnpackedKind::Inf {
        return (F80::zero(sign), flags);
    }
    if ua.kind == UnpackedKind::Zero && ub.kind == UnpackedKind::Zero {
        flags.invalid = true;
        return (F80::qnan(), flags);
    }
    if ub.kind == UnpackedKind::Zero {
        flags.div_zero = true;
        return (F80::infinity(sign), flags);
    }
    if ua.kind == UnpackedKind::Zero {
        return (F80::zero(sign), flags);
    }

    if a.is_denormal() || b.is_denormal() {
        flags.denormal = true;
    }

    let dividend = ua.mant;
    let divisor = ub.mant;

    let num = (dividend as u128) << 64;
    let q = num / (divisor as u128);
    let r = num % (divisor as u128);

    let (sig64, mut guard, exp_adjust) = if q >= (1u128 << 64) {
        ((q >> 1) as u64, (q & 1) != 0, 0i32)
    } else {
        (q as u64, false, -1i32)
    };

    let divisor128 = divisor as u128;
    let (round, sticky) = if exp_adjust == 0 {
        let (round_bit, rem) = first_frac_bit(r, divisor128);
        (round_bit, rem != 0)
    } else {
        let (g, rem1) = first_frac_bit(r, divisor128);
        guard = g;
        let (round_bit, rem2) = first_frac_bit(rem1, divisor128);
        (round_bit, rem2 != 0)
    };

    let sig = ((sig64 as u128) << 64) | ((guard as u128) << 63) | ((round as u128) << 62);

    let exp = ua.exp - ub.exp - 1 + exp_adjust;

    let result = round_pack(sign, exp, sig, sticky, rm, &mut flags);
    (result, flags)
}

fn isqrt_u128(n: u128) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut result = 0u128;
    let mut bit = 1u128 << 126;
    while bit > n {
        bit >>= 2;
    }
    while bit != 0 {
        if x >= result + bit {
            x -= result + bit;
            result = (result >> 1) + bit;
        } else {
            result >>= 1;
        }
        bit >>= 2;
    }
    result as u64
}

pub fn sqrt(a: F80, rm: RoundingMode) -> (F80, ExceptionFlags) {
    let mut flags = ExceptionFlags::default();
    let ua = unpack(&a);

    match ua.kind {
        UnpackedKind::QNaN => return (F80::qnan(), flags),
        UnpackedKind::SNaN => {
            flags.invalid = true;
            return (F80::qnan(), flags);
        }
        UnpackedKind::Inf => {
            if ua.sign {
                flags.invalid = true;
                return (F80::qnan(), flags);
            }
            return (F80::infinity(false), flags);
        }
        UnpackedKind::Zero => {
            return (F80::zero(ua.sign), flags);
        }
        UnpackedKind::Normal => {}
    }

    if ua.sign {
        flags.invalid = true;
        return (F80::qnan(), flags);
    }

    if a.is_denormal() {
        flags.denormal = true;
    }

    let mant = ua.mant;
    let e = ua.exp - 63;

    let (n, mut result_exp) = if e & 1 == 0 {
        ((mant as u128) << 64, e / 2 + 31)
    } else {
        ((mant as u128) << 63, (e - 1) / 2 + 32)
    };

    let q = isqrt_u128(n);
    let q2 = (q as u128) * (q as u128);
    let r = n - q2;

    let mut result_mant = q;
    if r > 0 {
        flags.precision = true;
        let increment = match rm {
            RoundingMode::NearestEven => r > (q as u128),
            RoundingMode::Up => true,
            RoundingMode::Down => false,
            RoundingMode::TowardZero => false,
        };
        if increment {
            if result_mant == u64::MAX {
                result_mant = 1 << 63;
                result_exp += 1;
            } else {
                result_mant += 1;
            }
        }
    }

    let biased = (result_exp + EXP_BIAS as i32) as u16;
    (F80::from_parts(false, biased, result_mant), flags)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpResult {
    Greater,
    Less,
    Equal,
    Unordered,
}

impl CmpResult {
    #[inline]
    pub fn condition_codes(self) -> (bool, bool, bool, bool) {
        match self {
            CmpResult::Greater => (false, false, false, false),
            CmpResult::Less => (false, false, false, true),
            CmpResult::Equal => (true, false, false, false),
            CmpResult::Unordered => (true, true, false, true),
        }
    }
}

pub fn cmp(a: F80, b: F80) -> (CmpResult, ExceptionFlags) {
    let mut flags = ExceptionFlags::default();
    let ua = unpack(&a);
    let ub = unpack(&b);
    let a_nan = matches!(ua.kind, UnpackedKind::QNaN | UnpackedKind::SNaN);
    let b_nan = matches!(ub.kind, UnpackedKind::QNaN | UnpackedKind::SNaN);
    if a_nan || b_nan {
        if ua.kind == UnpackedKind::SNaN || ub.kind == UnpackedKind::SNaN {
            flags.invalid = true;
        }
        return (CmpResult::Unordered, flags);
    }
    match (ua.kind, ub.kind) {
        (UnpackedKind::Inf, UnpackedKind::Inf) => {
            if ua.sign == ub.sign {
                return (CmpResult::Equal, flags);
            }

            return (
                if ua.sign {
                    CmpResult::Less
                } else {
                    CmpResult::Greater
                },
                flags,
            );
        }
        (UnpackedKind::Inf, _) => {
            return (
                if ua.sign {
                    CmpResult::Less
                } else {
                    CmpResult::Greater
                },
                flags,
            );
        }
        (_, UnpackedKind::Inf) => {
            return (
                if ub.sign {
                    CmpResult::Greater
                } else {
                    CmpResult::Less
                },
                flags,
            );
        }
        _ => {}
    }
    match (ua.kind, ub.kind) {
        (UnpackedKind::Zero, UnpackedKind::Zero) => {
            return (CmpResult::Equal, flags);
        }
        (UnpackedKind::Zero, _) => {
            return (
                if ub.sign {
                    CmpResult::Greater
                } else {
                    CmpResult::Less
                },
                flags,
            );
        }
        (_, UnpackedKind::Zero) => {
            return (
                if ua.sign {
                    CmpResult::Less
                } else {
                    CmpResult::Greater
                },
                flags,
            );
        }
        _ => {}
    }
    if a.is_denormal() || b.is_denormal() {
        flags.denormal = true;
    }
    if ua.sign != ub.sign {
        return (
            if ua.sign {
                CmpResult::Less
            } else {
                CmpResult::Greater
            },
            flags,
        );
    }
    let abs_cmp = if ua.exp != ub.exp {
        if ua.exp > ub.exp { 1 } else { -1 }
    } else if ua.mant != ub.mant {
        if ua.mant > ub.mant { 1 } else { -1 }
    } else {
        0
    };

    let result = if ua.sign {
        match abs_cmp {
            1 => CmpResult::Less,
            -1 => CmpResult::Greater,
            _ => CmpResult::Equal,
        }
    } else {
        match abs_cmp {
            1 => CmpResult::Greater,
            -1 => CmpResult::Less,
            _ => CmpResult::Equal,
        }
    };

    (result, flags)
}

pub fn ftst(a: F80) -> (CmpResult, ExceptionFlags) {
    cmp(a, F80::zero(false))
}

pub fn round_to_integer(a: F80, rm: RoundingMode) -> (F80, ExceptionFlags) {
    let mut flags = ExceptionFlags::default();
    let ua = unpack(&a);

    match ua.kind {
        UnpackedKind::Zero => return (F80::zero(ua.sign), flags),
        UnpackedKind::Inf => return (F80::infinity(ua.sign), flags),
        UnpackedKind::QNaN => return (F80::qnan(), flags),
        UnpackedKind::SNaN => {
            flags.invalid = true;
            return (F80::qnan(), flags);
        }
        UnpackedKind::Normal => {}
    }

    if a.is_denormal() {
        flags.denormal = true;
    }
    if ua.exp >= 63 {
        return (a, flags);
    }

    let shift = (63 - ua.exp) as u32;
    if shift >= 64 {
        let result = match rm {
            RoundingMode::NearestEven => {
                if ua.exp == -1 && ua.mant != (1u64 << 63) {
                    F80::one(ua.sign)
                } else {
                    F80::zero(ua.sign)
                }
            }
            RoundingMode::Down => {
                if ua.sign {
                    F80::one(true)
                } else {
                    F80::zero(ua.sign)
                }
            }
            RoundingMode::Up => {
                if ua.sign {
                    F80::zero(ua.sign)
                } else {
                    F80::one(false)
                }
            }
            RoundingMode::TowardZero => F80::zero(ua.sign),
        };
        flags.precision = true;
        return (result, flags);
    }

    let (int_part, guard, round, sticky) = shift_right_grs(ua.mant as u128, shift);
    let any_lost = guard || round || sticky;
    if any_lost {
        flags.precision = true;
    }

    let increment = match rm {
        RoundingMode::NearestEven => guard && (round || sticky || (int_part & 1 != 0)),
        RoundingMode::Down => ua.sign && any_lost,
        RoundingMode::Up => !ua.sign && any_lost,
        RoundingMode::TowardZero => false,
    };

    let mut int_val = int_part;
    if increment {
        int_val += 1;
    }

    if int_val == 0 {
        return (F80::zero(ua.sign), flags);
    }
    let int_u64 = int_val as u64;
    let lz = int_u64.leading_zeros() as i32;
    let mant = int_u64 << lz;
    let exp = 63 - lz;
    let biased = EXP_BIAS as i32 + exp;
    (F80::from_parts(ua.sign, biased as u16, mant), flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f64_to_f80(v: f64) -> F80 {
        let bits = v.to_bits();
        let sign = (bits >> 63) & 1 != 0;
        let f64_exp = ((bits >> 52) & 0x7FF) as i32;
        let f64_frac = bits & 0x000F_FFFF_FFFF_FFFF;
        if f64_exp == 0 && f64_frac == 0 {
            return F80::zero(sign);
        }
        if f64_exp == 0x7FF {
            return if f64_frac == 0 {
                F80::infinity(sign)
            } else {
                F80::qnan()
            };
        }
        let mant = (0x0010_0000_0000_0000u64 | f64_frac) << 11;
        let exp = f64_exp - 1023;
        let biased = (exp + EXP_BIAS as i32) as u16;
        F80::from_parts(sign, biased, mant)
    }

    #[test]
    fn test_one_plus_one() {
        let one = f64_to_f80(1.0);
        let (result, _) = add(one, one, RoundingMode::NearestEven);
        let two = f64_to_f80(2.0);
        assert_eq!(result, two);
    }

    #[test]
    fn test_sub_equal() {
        let one = f64_to_f80(1.0);
        let (result, _) = sub(one, one, RoundingMode::NearestEven);
        assert_eq!(result.classify(), F80Class::Zero);
    }

    #[test]
    fn test_add_opposite_signs() {
        let one = f64_to_f80(1.0);
        let neg_one = neg(one);
        let (result, _) = add(one, neg_one, RoundingMode::NearestEven);
        assert_eq!(result.classify(), F80Class::Zero);
    }

    #[test]
    fn test_half_plus_half() {
        let half = f64_to_f80(0.5);
        let (result, _) = add(half, half, RoundingMode::NearestEven);
        let one = f64_to_f80(1.0);
        assert_eq!(result, one);
    }

    #[test]
    fn test_add_zero() {
        let one = f64_to_f80(1.0);
        let zero = F80::zero(false);
        let (result, _) = add(one, zero, RoundingMode::NearestEven);
        assert_eq!(result, one);
    }

    #[test]
    fn test_add_infinity() {
        let one = f64_to_f80(1.0);
        let inf = F80::infinity(false);
        let (result, _) = add(one, inf, RoundingMode::NearestEven);
        assert_eq!(result.classify(), F80Class::Infinity);
    }

    #[test]
    fn test_inf_plus_neg_inf() {
        let inf = F80::infinity(false);
        let neg_inf = F80::infinity(true);
        let (result, flags) = add(inf, neg_inf, RoundingMode::NearestEven);
        assert!(result.is_nan());
        assert!(flags.invalid);
    }

    #[test]
    fn test_add_nan() {
        let one = f64_to_f80(1.0);
        let nan = F80::qnan();
        let (result, _) = add(one, nan, RoundingMode::NearestEven);
        assert!(result.is_nan());
    }

    #[test]
    fn test_snan_sets_invalid() {
        let one = f64_to_f80(1.0);
        let snan = F80::snan();
        let (result, flags) = add(one, snan, RoundingMode::NearestEven);
        assert!(result.is_nan());
        assert!(flags.invalid);
    }

    #[test]
    fn test_negation() {
        let one = f64_to_f80(1.0);
        let neg_one = neg(one);
        assert!(neg_one.sign());
        assert_eq!(neg_one.biased_exp(), one.biased_exp());
        assert_eq!(neg_one.mant, one.mant);
    }

    #[test]
    fn test_different_exponents() {
        let a = f64_to_f80(1.5);
        let b = f64_to_f80(0.25);
        let (result, _) = add(a, b, RoundingMode::NearestEven);
        let expected = f64_to_f80(1.75);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_subtraction_larger() {
        let two = f64_to_f80(2.0);
        let one = f64_to_f80(1.0);
        let (result, _) = sub(two, one, RoundingMode::NearestEven);
        assert_eq!(result, one);
    }

    #[test]
    fn test_mul_one_one() {
        let one = f64_to_f80(1.0);
        let (result, _) = mul(one, one, RoundingMode::NearestEven);
        assert_eq!(result, one);
    }

    #[test]
    fn test_mul_two_three() {
        let two = f64_to_f80(2.0);
        let three = f64_to_f80(3.0);
        let (result, _) = mul(two, three, RoundingMode::NearestEven);
        let six = f64_to_f80(6.0);
        assert_eq!(result, six);
    }

    #[test]
    fn test_mul_fraction() {
        let half = f64_to_f80(0.5);
        let (result, _) = mul(half, half, RoundingMode::NearestEven);
        let quarter = f64_to_f80(0.25);
        assert_eq!(result, quarter);
    }

    #[test]
    fn test_mul_neg() {
        let one = f64_to_f80(1.0);
        let neg_one = neg(one);
        let (result, _) = mul(one, neg_one, RoundingMode::NearestEven);
        assert_eq!(result, neg_one);
        assert!(result.sign());
    }

    #[test]
    fn test_mul_neg_neg() {
        let neg_one = neg(f64_to_f80(1.0));
        let (result, _) = mul(neg_one, neg_one, RoundingMode::NearestEven);
        let one = f64_to_f80(1.0);
        assert_eq!(result, one);
        assert!(!result.sign());
    }

    #[test]
    fn test_mul_zero() {
        let five = f64_to_f80(5.0);
        let zero = F80::zero(false);
        let (result, _) = mul(five, zero, RoundingMode::NearestEven);
        assert_eq!(result.classify(), F80Class::Zero);
    }

    #[test]
    fn test_mul_inf() {
        let two = f64_to_f80(2.0);
        let inf = F80::infinity(false);
        let (result, _) = mul(two, inf, RoundingMode::NearestEven);
        assert_eq!(result.classify(), F80Class::Infinity);
    }

    #[test]
    fn test_mul_inf_zero() {
        let inf = F80::infinity(false);
        let zero = F80::zero(false);
        let (result, flags) = mul(inf, zero, RoundingMode::NearestEven);
        assert!(result.is_nan());
        assert!(flags.invalid);
    }

    #[test]
    fn test_mul_nan() {
        let one = f64_to_f80(1.0);
        let nan = F80::qnan();
        let (result, _) = mul(one, nan, RoundingMode::NearestEven);
        assert!(result.is_nan());
    }

    #[test]
    fn test_div_one_one() {
        let one = f64_to_f80(1.0);
        let (result, _) = div(one, one, RoundingMode::NearestEven);
        assert_eq!(result, one);
    }

    #[test]
    fn test_div_two_one() {
        let two = f64_to_f80(2.0);
        let one = f64_to_f80(1.0);
        let (result, _) = div(two, one, RoundingMode::NearestEven);
        assert_eq!(result, two);
    }

    #[test]
    fn test_div_one_two() {
        let one = f64_to_f80(1.0);
        let two = f64_to_f80(2.0);
        let (result, _) = div(one, two, RoundingMode::NearestEven);
        let half = f64_to_f80(0.5);
        assert_eq!(result, half);
    }

    #[test]
    fn test_div_six_two() {
        let six = f64_to_f80(6.0);
        let two = f64_to_f80(2.0);
        let (result, _) = div(six, two, RoundingMode::NearestEven);
        let three = f64_to_f80(3.0);
        assert_eq!(result, three);
    }

    #[test]
    fn test_div_neg() {
        let one = f64_to_f80(1.0);
        let neg_one = neg(one);
        let (result, _) = div(neg_one, one, RoundingMode::NearestEven);
        assert_eq!(result, neg_one);
        assert!(result.sign());
    }

    #[test]
    fn test_div_by_zero() {
        let one = f64_to_f80(1.0);
        let zero = F80::zero(false);
        let (result, flags) = div(one, zero, RoundingMode::NearestEven);
        assert_eq!(result.classify(), F80Class::Infinity);
        assert!(flags.div_zero);
    }

    #[test]
    fn test_zero_div_zero() {
        let zero = F80::zero(false);
        let (result, flags) = div(zero, zero, RoundingMode::NearestEven);
        assert!(result.is_nan());
        assert!(flags.invalid);
    }

    #[test]
    fn test_inf_div_inf() {
        let inf = F80::infinity(false);
        let (result, flags) = div(inf, inf, RoundingMode::NearestEven);
        assert!(result.is_nan());
        assert!(flags.invalid);
    }

    #[test]
    fn test_inf_div_one() {
        let inf = F80::infinity(false);
        let one = f64_to_f80(1.0);
        let (result, _) = div(inf, one, RoundingMode::NearestEven);
        assert_eq!(result.classify(), F80Class::Infinity);
    }

    #[test]
    fn test_one_div_inf() {
        let one = f64_to_f80(1.0);
        let inf = F80::infinity(false);
        let (result, _) = div(one, inf, RoundingMode::NearestEven);
        assert_eq!(result.classify(), F80Class::Zero);
    }

    #[test]
    fn test_div_nan() {
        let one = f64_to_f80(1.0);
        let nan = F80::qnan();
        let (result, _) = div(one, nan, RoundingMode::NearestEven);
        assert!(result.is_nan());
    }

    #[test]
    fn test_sqrt_one() {
        let (r, _) = sqrt(f64_to_f80(1.0), RoundingMode::NearestEven);
        assert_eq!(r, f64_to_f80(1.0));
    }

    #[test]
    fn test_sqrt_four() {
        let (r, _) = sqrt(f64_to_f80(4.0), RoundingMode::NearestEven);
        assert_eq!(r, f64_to_f80(2.0));
    }

    #[test]
    fn test_sqrt_nine() {
        let (r, _) = sqrt(f64_to_f80(9.0), RoundingMode::NearestEven);
        assert_eq!(r, f64_to_f80(3.0));
    }

    #[test]
    fn test_sqrt_quarter() {
        let (r, _) = sqrt(f64_to_f80(0.25), RoundingMode::NearestEven);
        assert_eq!(r, f64_to_f80(0.5));
    }

    #[test]
    fn test_sqrt_zero() {
        let (r, _) = sqrt(F80::zero(false), RoundingMode::NearestEven);
        assert_eq!(r.classify(), F80Class::Zero);
    }

    #[test]
    fn test_sqrt_neg_invalid() {
        let (r, flags) = sqrt(neg(f64_to_f80(1.0)), RoundingMode::NearestEven);
        assert!(r.is_nan());
        assert!(flags.invalid);
    }

    #[test]
    fn test_sqrt_inf() {
        let (r, _) = sqrt(F80::infinity(false), RoundingMode::NearestEven);
        assert_eq!(r.classify(), F80Class::Infinity);
    }

    #[test]
    fn test_sqrt_neg_inf_invalid() {
        let (r, flags) = sqrt(F80::infinity(true), RoundingMode::NearestEven);
        assert!(r.is_nan());
        assert!(flags.invalid);
    }

    #[test]
    fn test_sqrt_nan() {
        let (r, _) = sqrt(F80::qnan(), RoundingMode::NearestEven);
        assert!(r.is_nan());
    }

    #[test]
    fn test_sqrt_two_irrational() {
        let (r, flags) = sqrt(f64_to_f80(2.0), RoundingMode::NearestEven);
        assert_eq!(r.classify(), F80Class::Normal);
        assert!(!r.sign());
        assert_eq!(r.biased_exp(), 16383);
        assert!(flags.precision);
    }

    #[test]
    fn test_cmp_greater() {
        let two = f64_to_f80(2.0);
        let one = f64_to_f80(1.0);
        let (r, _) = cmp(two, one);
        assert_eq!(r, CmpResult::Greater);
    }

    #[test]
    fn test_cmp_less() {
        let one = f64_to_f80(1.0);
        let two = f64_to_f80(2.0);
        let (r, _) = cmp(one, two);
        assert_eq!(r, CmpResult::Less);
    }

    #[test]
    fn test_cmp_equal() {
        let one = f64_to_f80(1.0);
        let (r, _) = cmp(one, one);
        assert_eq!(r, CmpResult::Equal);
    }

    #[test]
    fn test_cmp_zero_equal() {
        let pz = F80::zero(false);
        let nz = F80::zero(true);
        let (r, _) = cmp(pz, nz);
        assert_eq!(r, CmpResult::Equal);
    }

    #[test]
    fn test_cmp_neg_values() {
        let neg_one = neg(f64_to_f80(1.0));
        let neg_two = neg(f64_to_f80(2.0));
        let (r, _) = cmp(neg_one, neg_two);
        assert_eq!(r, CmpResult::Greater);
    }

    #[test]
    fn test_cmp_diff_signs() {
        let one = f64_to_f80(1.0);
        let neg_one = neg(one);
        let (r, _) = cmp(one, neg_one);
        assert_eq!(r, CmpResult::Greater);
        let (r, _) = cmp(neg_one, one);
        assert_eq!(r, CmpResult::Less);
    }

    #[test]
    fn test_cmp_nan_unordered() {
        let one = f64_to_f80(1.0);
        let nan = F80::qnan();
        let (r, _) = cmp(one, nan);
        assert_eq!(r, CmpResult::Unordered);
        let (r, _) = cmp(nan, one);
        assert_eq!(r, CmpResult::Unordered);
    }

    #[test]
    fn test_cmp_snan_invalid() {
        let one = f64_to_f80(1.0);
        let snan = F80::snan();
        let (r, flags) = cmp(one, snan);
        assert_eq!(r, CmpResult::Unordered);
        assert!(flags.invalid);
    }

    #[test]
    fn test_cmp_inf() {
        let inf = F80::infinity(false);
        let one = f64_to_f80(1.0);
        let (r, _) = cmp(inf, one);
        assert_eq!(r, CmpResult::Greater);
        let (r, _) = cmp(one, inf);
        assert_eq!(r, CmpResult::Less);
    }

    #[test]
    fn test_cmp_neg_inf() {
        let ninf = F80::infinity(true);
        let one = f64_to_f80(1.0);
        let (r, _) = cmp(ninf, one);
        assert_eq!(r, CmpResult::Less);
    }

    #[test]
    fn test_cmp_inf_inf() {
        let inf = F80::infinity(false);
        let (r, _) = cmp(inf, inf);
        assert_eq!(r, CmpResult::Equal);
        let ninf = F80::infinity(true);
        let (r, _) = cmp(inf, ninf);
        assert_eq!(r, CmpResult::Greater);
        let (r, _) = cmp(ninf, inf);
        assert_eq!(r, CmpResult::Less);
    }

    #[test]
    fn test_cmp_inf_vs_inf_diff_sign() {
        let inf = F80::infinity(false);
        let ninf = F80::infinity(true);
        let (r, _) = cmp(inf, ninf);
        assert_eq!(r, CmpResult::Greater);
    }

    #[test]
    fn test_ftst_positive() {
        let one = f64_to_f80(1.0);
        let (r, _) = ftst(one);
        assert_eq!(r, CmpResult::Greater);
    }

    #[test]
    fn test_ftst_negative() {
        let neg_one = neg(f64_to_f80(1.0));
        let (r, _) = ftst(neg_one);
        assert_eq!(r, CmpResult::Less);
    }

    #[test]
    fn test_ftst_zero() {
        let z = F80::zero(false);
        let (r, _) = ftst(z);
        assert_eq!(r, CmpResult::Equal);
    }

    #[test]
    fn test_ftst_nan() {
        let nan = F80::qnan();
        let (r, _) = ftst(nan);
        assert_eq!(r, CmpResult::Unordered);
    }

    #[test]
    fn test_condition_codes() {
        let (c3, c2, c1, c0) = CmpResult::Greater.condition_codes();
        assert_eq!((c3, c2, c1, c0), (false, false, false, false));

        let (c3, c2, c1, c0) = CmpResult::Less.condition_codes();
        assert_eq!((c3, c2, c1, c0), (false, false, false, true));

        let (c3, c2, c1, c0) = CmpResult::Equal.condition_codes();
        assert_eq!((c3, c2, c1, c0), (true, false, false, false));

        let (c3, c2, c1, c0) = CmpResult::Unordered.condition_codes();
        assert_eq!((c3, c2, c1, c0), (true, true, false, true));
    }
}
