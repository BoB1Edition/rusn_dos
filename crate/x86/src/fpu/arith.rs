// Ver: 1 File: crate/x86/src/fpu/arith.rs
//! Точная арифметика 80-битного расширенного формата.
//! Сложение/вычитание с корректным округлением во всех 4 режимах.

use super::f80::{EXP_BIAS, EXP_MAX, F80, F80Class};

/// Режим округления (биты 10-11 контрольного слова).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundingMode {
    NearestEven, // 00 — к ближайшему, ничья к чётному
    Down,        // 01 — к -∞
    Up,          // 10 — к +∞
    TowardZero,  // 11 — усечение
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

/// Флаги исключений (биты 0-5 статусного слова).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExceptionFlags {
    pub invalid: bool,   // IE
    pub denormal: bool,  // DE
    pub div_zero: bool,  // ZE
    pub overflow: bool,  // OE
    pub underflow: bool, // UE
    pub precision: bool, // PE
}

/// Несмещённая экспонента минимального нормального числа (смещённая = 1).
const MIN_NORMAL_EXP: i32 = 1 - EXP_BIAS as i32; // -16382
/// Несмещённая экспонента максимального конечного числа (смещённая = 0x7FFE).
const MAX_NORMAL_EXP: i32 = (EXP_MAX as i32 - 1) - EXP_BIAS as i32; // 16383

/// Внутреннее распакованное представление для точной арифметики.
#[derive(Clone, Copy, Debug)]
struct Unpacked {
    sign: bool,
    exp: i32,  // несмещённая экспонента (для нормализованной мантиссы)
    mant: u64, // нормализованная мантисса, бит 63 = 1 для ненулевых конечных
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

/// Распаковывает `F80` во внутреннее представление, нормализуя денормалы.
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
        // Normal, Subnormal, Unnormal, Denormal
        _ => {
            if mant == 0 {
                return Unpacked {
                    sign,
                    exp: 0,
                    mant: 0,
                    kind: UnpackedKind::Zero,
                };
            }
            // Нормализуем: сдвигаем влево до появления 1 в бите 63.
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


/// Сдвигает `sig` вправо на `shift`, возвращая (результат, sticky).
/// sticky = true, если любой из отброшенных битов был 1.
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

/// Сдвиг вправо с возвратом (результат, guard, round, sticky).
/// guard = бит (n-1), round = бит (n-2), sticky = OR битов 0..(n-3).
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

/// Округляет и упаковывает результат.
/// `sig` — ненормализованная мантисса в `u128` (целочисленный бит в бите 126),
/// `exp` — экспонента до нормализации, `sticky` — потерянные при выравнивании биты.
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

    // Нормализация: старшая 1 в бит 127.
    let lz = sig.leading_zeros() as i32;
    sig <<= lz;
    exp -= lz;

    // Целочисленный бит нормализованного sig в бите 127, а в 64-битной
    // мантиссе (sig >> 64) он окажется в бите 63. Это даёт множитель 2,
    // поэтому несмещённая экспонента F80 = exp + 1.
    let mut f80_exp = exp + 1;

    // Значащие 64 бита: биты 64-127. Защитные: биты 0-63.
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
            f80_exp += 1; // перенос из округления
        } else {
            sig64 += 1;
        }
    }
    if any_lost {
        flags.precision = true;
    }

    // Переполнение.
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

    // Исчезновение порядка (денормал или ноль).
    if f80_exp < MIN_NORMAL_EXP {
        // Значение = sig * 2^(f80_exp - 127).
        // Денормал: значение = m * 2^(MIN_NORMAL_EXP - 63), m < 2^63.
        // m = sig >> shift, где shift = 64 + (MIN_NORMAL_EXP - f80_exp).
        let shift = (MIN_NORMAL_EXP - f80_exp) as u32 + 64;
        let (shifted, d_guard, d_round, d_sticky_bits) = shift_right_grs(sig, shift);
        let mut denorm_mant = shifted as u64; // старшая 1 ниже бита 63
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
        }
        flags.underflow = true;

        return if denorm_mant & (1 << 63) != 0 {
            // Округление подняло денормал до минимального нормального.
            F80::from_parts(sign, 1, denorm_mant)
        } else {
            F80::from_parts(sign, 0, denorm_mant)
        };
    }

    // Нормальный результат.
    let biased = (f80_exp + EXP_BIAS as i32) as u16;
    F80::from_parts(sign, biased, sig64)
}

/// Округляет одно распакованное число (используется, когда второй операнд — ноль).
fn round_single(u: &Unpacked, rm: RoundingMode, flags: &mut ExceptionFlags) -> F80 {
    if u.kind == UnpackedKind::Zero {
        return F80::zero(u.sign);
    }
    let sig = (u.mant as u128) << 63;
    round_pack(u.sign, u.exp, sig, false, rm, flags)
}

/// Сложение двух `F80`.
pub fn add(a: F80, b: F80, rm: RoundingMode) -> (F80, ExceptionFlags) {
    let mut flags = ExceptionFlags::default();
    let ua = unpack(&a);
    let ub = unpack(&b);

    // --- NaN ---
    let a_nan = matches!(ua.kind, UnpackedKind::QNaN | UnpackedKind::SNaN);
    let b_nan = matches!(ub.kind, UnpackedKind::QNaN | UnpackedKind::SNaN);
    if a_nan || b_nan {
        if ua.kind == UnpackedKind::SNaN || ub.kind == UnpackedKind::SNaN {
            flags.invalid = true;
        }
        return (F80::qnan(), flags);
    }

    // --- Infinity ---
    match (ua.kind, ub.kind) {
        (UnpackedKind::Inf, UnpackedKind::Inf) => {
            if ua.sign != ub.sign {
                flags.invalid = true; // ∞ + (-∞) = NaN
                return (F80::qnan(), flags);
            }
            return (F80::infinity(ua.sign), flags);
        }
        (UnpackedKind::Inf, _) => return (F80::infinity(ua.sign), flags),
        (_, UnpackedKind::Inf) => return (F80::infinity(ub.sign), flags),
        _ => {}
    }

    // --- Zero ---
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

    // Оба — нормальные конечные числа.
    if a.is_denormal() || b.is_denormal() {
        flags.denormal = true;
    }

    // Расширяем мантиссы: целочисленный бит в бите 126 (запас для переноса в бит 127).
    let mut a_sig = (ua.mant as u128) << 63;
    let mut b_sig = (ub.mant as u128) << 63;
    let mut a_exp = ua.exp;
    let mut b_exp = ub.exp;
    let mut a_sign = ua.sign;
    let mut b_sign = ub.sign;

    // Выравнивание: сдвигаем мантиссу с меньшей экспонентой вправо.
    if a_exp < b_exp {
        std::mem::swap(&mut a_sig, &mut b_sig);
        std::mem::swap(&mut a_exp, &mut b_exp);
        std::mem::swap(&mut a_sign, &mut b_sign);
    }
    let d = (a_exp - b_exp) as u32;
    let (shifted, mut sticky) = shift_right_sticky(b_sig, d);
    b_sig = shifted;

    // Сложение или вычитание мантисс.
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

/// Инверсия знака.
pub fn neg(f: F80) -> F80 {
    F80::from_parts(!f.sign(), f.biased_exp(), f.mant)
}

/// Вычитание двух `F80`.
pub fn sub(a: F80, b: F80, rm: RoundingMode) -> (F80, ExceptionFlags) {
    add(a, neg(b), rm)
}

/// Умножение двух `F80`.
pub fn mul(a: F80, b: F80, rm: RoundingMode) -> (F80, ExceptionFlags) {
    let mut flags = ExceptionFlags::default();
    let ua = unpack(&a);
    let ub = unpack(&b);

    // --- NaN ---
    let a_nan = matches!(ua.kind, UnpackedKind::QNaN | UnpackedKind::SNaN);
    let b_nan = matches!(ub.kind, UnpackedKind::QNaN | UnpackedKind::SNaN);
    if a_nan || b_nan {
        if ua.kind == UnpackedKind::SNaN || ub.kind == UnpackedKind::SNaN {
            flags.invalid = true;
        }
        return (F80::qnan(), flags);
    }

    // --- Infinity и Zero ---
    match (ua.kind, ub.kind) {
        // ∞ × 0 = NaN (invalid) — важное отличие от сложения
        (UnpackedKind::Inf, UnpackedKind::Zero) | (UnpackedKind::Zero, UnpackedKind::Inf) => {
            flags.invalid = true;
            return (F80::qnan(), flags);
        }
        // ∞ × ненулевое = ∞, знак = XOR знаков
        (UnpackedKind::Inf, _) | (_, UnpackedKind::Inf) => {
            return (F80::infinity(ua.sign ^ ub.sign), flags);
        }
        // 0 × конечное = 0, знак = XOR знаков
        (UnpackedKind::Zero, _) | (_, UnpackedKind::Zero) => {
            return (F80::zero(ua.sign ^ ub.sign), flags);
        }
        _ => {}
    }

    // Оба — нормальные конечные числа.
    if a.is_denormal() || b.is_denormal() {
        flags.denormal = true;
    }

    // Перемножение мантисс: 64 × 64 → 128 бит (без переполнения).
    let product = (ua.mant as u128) * (ub.mant as u128);

    // Экспоненты складываются.
    let exp = ua.exp + ub.exp;

    let result = round_pack(ua.sign ^ ub.sign, exp, product, false, rm, &mut flags);
    (result, flags)
}

/// Возвращает первый бит дроби r/d и остаток для извлечения следующего бита.
/// Предполагается r < d. Результат: (бит, новый_остаток).
fn first_frac_bit(r: u128, d: u128) -> (bool, u128) {
    let twice = r << 1;
    if twice >= d {
        (true, twice - d)
    } else {
        (false, twice)
    }
}

/// Деление двух `F80`.
pub fn div(a: F80, b: F80, rm: RoundingMode) -> (F80, ExceptionFlags) {
    let mut flags = ExceptionFlags::default();
    let ua = unpack(&a);
    let ub = unpack(&b);

    // --- NaN ---
    let a_nan = matches!(ua.kind, UnpackedKind::QNaN | UnpackedKind::SNaN);
    let b_nan = matches!(ub.kind, UnpackedKind::QNaN | UnpackedKind::SNaN);
    if a_nan || b_nan {
        if ua.kind == UnpackedKind::SNaN || ub.kind == UnpackedKind::SNaN {
            flags.invalid = true;
        }
        return (F80::qnan(), flags);
    }

    let sign = ua.sign ^ ub.sign;

    // --- ∞ / ∞ = NaN (invalid) ---
    if ua.kind == UnpackedKind::Inf && ub.kind == UnpackedKind::Inf {
        flags.invalid = true;
        return (F80::qnan(), flags);
    }
    // --- ∞ / конечное = ∞ ---
    if ua.kind == UnpackedKind::Inf {
        return (F80::infinity(sign), flags);
    }
    // --- конечное / ∞ = 0 ---
    if ub.kind == UnpackedKind::Inf {
        return (F80::zero(sign), flags);
    }
    // --- 0 / 0 = NaN (invalid) ---
    if ua.kind == UnpackedKind::Zero && ub.kind == UnpackedKind::Zero {
        flags.invalid = true;
        return (F80::qnan(), flags);
    }
    // --- ненулевое / 0 = ∞ (деление на ноль) ---
    if ub.kind == UnpackedKind::Zero {
        flags.div_zero = true;
        return (F80::infinity(sign), flags);
    }
    // --- 0 / конечное = 0 ---
    if ua.kind == UnpackedKind::Zero {
        return (F80::zero(sign), flags);
    }

    // Оба — нормальные конечные числа.
    if a.is_denormal() || b.is_denormal() {
        flags.denormal = true;
    }

    let dividend = ua.mant;
    let divisor = ub.mant;

    // Делим мантиссы: обе имеют бит 63 = 1.
    // q = (dividend / divisor) * 2^64, частное в (0.5, 2), значит q в (2^63, 2^65).
    let num = (dividend as u128) << 64;
    let q = num / (divisor as u128);
    let r = num % (divisor as u128);

    let (sig64, mut guard, exp_adjust) = if q >= (1u128 << 64) {
        // частное >= 1: q 65 бит. Мантисса = q>>1, guard = младший бит q.
        ((q >> 1) as u64, (q & 1) != 0, 0i32)
    } else {
        // частное < 1: q 64 бита = уже нормализованная мантисса, экспонента -1.
        (q as u64, false, -1i32)
    };

    // Извлекаем недостающие биты (round, и иногда guard) из остатка r.
    let divisor128 = divisor as u128;
    let (round, sticky) = if exp_adjust == 0 {
        // guard уже взят из q; из r нужен round + sticky.
        let (round_bit, rem) = first_frac_bit(r, divisor128);
        (round_bit, rem != 0)
    } else {
        // guard берём из r, затем round + sticky.
        let (g, rem1) = first_frac_bit(r, divisor128);
        guard = g;
        let (round_bit, rem2) = first_frac_bit(rem1, divisor128);
        (round_bit, rem2 != 0)
    };

    // Конструируем sig для round_pack: мантисса в битах 64-127,
    // guard в 63, round в 62. Уже нормализован (бит 127 = 1).
    let sig = ((sig64 as u128) << 64) | ((guard as u128) << 63) | ((round as u128) << 62);

    let exp = ua.exp - ub.exp - 1 + exp_adjust;

    let result = round_pack(sign, exp, sig, sticky, rm, &mut flags);
    (result, flags)
}

/// Целочисленный квадратный корень из u128, результат в u64.
/// Вход до 128 бит, корень до 64 бит. Побитовый алгоритм.
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

/// Квадратный корень `F80`.
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
                // sqrt(-∞) = NaN, invalid
                flags.invalid = true;
                return (F80::qnan(), flags);
            }
            return (F80::infinity(false), flags);
        }
        UnpackedKind::Zero => {
            // sqrt(±0) = ±0
            return (F80::zero(ua.sign), flags);
        }
        UnpackedKind::Normal => {}
    }

    // Отрицательное ненулевое → NaN, invalid
    if ua.sign {
        flags.invalid = true;
        return (F80::qnan(), flags);
    }

    if a.is_denormal() {
        flags.denormal = true;
    }

    let mant = ua.mant; // бит 63 = 1
    // значение = mant × 2^e, где e = ua.exp - 63
    let e = ua.exp - 63;

    // Делаем экспоненту чётной, чтобы после деления пополам получить
    // нормализованную мантиссу. Расширяем мантиссу до 127/128 бит.
    let (n, mut result_exp) = if e & 1 == 0 {
        // e чётное: result_mant = isqrt(mant << 64)
        ((mant as u128) << 64, e / 2 + 31)
    } else {
        // e нечётное: result_mant = isqrt(mant << 63), экспонента на 1 меньше
        ((mant as u128) << 63, (e - 1) / 2 + 32)
    };

    let q = isqrt_u128(n);
    let q2 = (q as u128) * (q as u128);
    let r = n - q2; // остаток: 0 → точный корень

    let mut result_mant = q;
    if r > 0 {
        flags.precision = true;
        // Точного совпадения с серединой (r = q + 0.25) для целого r не бывает,
        // поэтому «ничья к чётному» не возникает — только ближайшее.
        let increment = match rm {
            RoundingMode::NearestEven => r > (q as u128), // sqrt > q + 0.5
            RoundingMode::Up => true,                     // любой остаток → вверх
            RoundingMode::Down => false,                  // sqrt ≥ 0 → вниз = усечение
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

/// Результат сравнения двух `F80`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpResult {
    Greater,
    Less,
    Equal,
    Unordered,
}

impl CmpResult {
    /// Биты C3, C2, C1, C0 для статусного слова FCOM/FCOMP/FCOMPP/FTST.
    #[inline]
    pub fn condition_codes(self) -> (bool, bool, bool, bool) {
        match self {
            CmpResult::Greater => (false, false, false, false),
            CmpResult::Less => (false, false, false, true),
            CmpResult::Equal => (true, false, false, false),
            CmpResult::Unordered => (true, false, false, true),
        }
    }
}

/// Сравнение двух `F80`.
pub fn cmp(a: F80, b: F80) -> (CmpResult, ExceptionFlags) {
    let mut flags = ExceptionFlags::default();
    let ua = unpack(&a);
    let ub = unpack(&b);

    // --- NaN ---
    let a_nan = matches!(ua.kind, UnpackedKind::QNaN | UnpackedKind::SNaN);
    let b_nan = matches!(ub.kind, UnpackedKind::QNaN | UnpackedKind::SNaN);
    if a_nan || b_nan {
        if ua.kind == UnpackedKind::SNaN || ub.kind == UnpackedKind::SNaN {
            flags.invalid = true;
        }
        return (CmpResult::Unordered, flags);
    }

    // --- Infinity ---
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

    // --- Zero ---
    match (ua.kind, ub.kind) {
        (UnpackedKind::Zero, UnpackedKind::Zero) => {
            // +0 = -0 в x87
            return (CmpResult::Equal, flags);
        }
        (UnpackedKind::Zero, _) => {
            // 0 vs b: если b > 0 → Less, если b < 0 → Greater
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

    // Оба — нормальные конечные числа.
    if a.is_denormal() || b.is_denormal() {
        flags.denormal = true;
    }

    // Разные знаки: положительный > отрицательный.
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

    // Одинаковые знаки: сравниваем по абсолютной величине.
    let abs_cmp = if ua.exp != ub.exp {
        if ua.exp > ub.exp { 1 } else { -1 }
    } else if ua.mant != ub.mant {
        if ua.mant > ub.mant { 1 } else { -1 }
    } else {
        0
    };

    let result = if ua.sign {
        // Оба отрицательные: инвертируем.
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

/// Сравнение `F80` с нулём (FTST).
pub fn ftst(a: F80) -> (CmpResult, ExceptionFlags) {
    cmp(a, F80::zero(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f64_to_f80(v: f64) -> F80 {
        // Простейшая конверсия для тестов через биты.
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
        // Нормальное f64: значение = (1 + frac/2^52) * 2^(f64_exp - 1023)
        // В 80-бит: мантисса = (2^52 + frac) << 11, целочисленный бит в бите 63.
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
        // 1.5 + 0.25 = 1.75
        let a = f64_to_f80(1.5);
        let b = f64_to_f80(0.25);
        let (result, _) = add(a, b, RoundingMode::NearestEven);
        let expected = f64_to_f80(1.75);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_subtraction_larger() {
        // 2.0 - 1.0 = 1.0
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
        // sqrt(2) ∈ [1, 2) → несмещённая экспонента 0, смещённая 16383
        assert_eq!(r.biased_exp(), 16383);
        // sqrt(2) иррационален → округление неточное
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
        assert_eq!(r, CmpResult::Equal); // +0 == -0
    }

    #[test]
    fn test_cmp_neg_values() {
        let neg_one = neg(f64_to_f80(1.0));
        let neg_two = neg(f64_to_f80(2.0));
        // -1 > -2
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
        assert_eq!((c3, c2, c1, c0), (true, false, false, true));
    }
}
