// Ver: 4

pub const CF: u16 = 1 << 0;
pub const PF: u16 = 1 << 2;
pub const AF: u16 = 1 << 4;
pub const ZF: u16 = 1 << 6;
pub const SF: u16 = 1 << 7;
pub const OF: u16 = 1 << 11;
pub const IF: u16 = 1 << 9;
pub const DF: u16 = 1 << 10;

// Добавлены недостающие флаги для полноты архитектуры
pub const TF: u16 = 1 << 8;
pub const IOPL: u16 = 3 << 12;
pub const NT: u16 = 1 << 14;

// Маска флагов, которые изменяются арифметическими и логическими операциями.
// Все остальные флаги (IF, DF, TF, IOPL, NT) должны сохраняться.
const ARITHMETIC_LOGIC_MASK: u16 = CF | PF | AF | ZF | SF | OF;

#[inline]
pub fn compute_flags_u8(current_flags: u16, result: u8, cf: bool, of: bool, af: bool) -> u16 {
    compute_flags_impl(current_flags, result as u32, 8, cf, of, af)
}

#[inline]
pub fn compute_flags_u16(current_flags: u16, result: u16, cf: bool, of: bool, af: bool) -> u16 {
    compute_flags_impl(current_flags, result as u32, 16, cf, of, af)
}

#[inline]
pub fn compute_flags_u32(current_flags: u16, result: u32, cf: bool, of: bool, af: bool) -> u16 {
    compute_flags_impl(current_flags, result, 32, cf, of, af)
}

#[inline]
pub(crate) fn compute_logical_flags_u8(current_flags: u16, result: u8) -> u16 {
    compute_flags_u8(current_flags, result, false, false, false)
}

#[inline]
pub fn compute_logical_flags_u16(current_flags: u16, result: u16) -> u16 {
    compute_flags_u16(current_flags, result, false, false, false)
}

#[inline]
pub fn compute_logical_flags_u32(current_flags: u16, result: u32) -> u16 {
    compute_flags_u32(current_flags, result, false, false, false)
}

#[inline]
fn compute_flags_impl(current_flags: u16, value: u32, width_bits: u32, cf: bool, of: bool, af: bool) -> u16 {
    let mut flags = current_flags & !ARITHMETIC_LOGIC_MASK;
    
    // Устанавливаем новые значения только для арифметических/логических флагов
    if value == 0 {
        flags |= ZF;
    }
    let sign_bit = 1u32 << (width_bits - 1);
    if (value & sign_bit) != 0 {
        flags |= SF;
    }
    if (value as u8).count_ones() % 2 == 0 {
        flags |= PF;
    }
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

#[inline]
pub fn test_flag(flags: u16, bit_mask: u16) -> bool {
    (flags & bit_mask) != 0
}

#[inline]
pub fn test_cf(flags: u16) -> bool { test_flag(flags, CF) }
#[inline]
pub fn test_pf(flags: u16) -> bool { test_flag(flags, PF) }
#[inline]
pub fn test_af(flags: u16) -> bool { test_flag(flags, AF) }
#[inline]
pub fn test_zf(flags: u16) -> bool { test_flag(flags, ZF) }
#[inline]
pub fn test_sf(flags: u16) -> bool { test_flag(flags, SF) }
#[inline]
pub fn test_of(flags: u16) -> bool { test_flag(flags, OF) }
#[inline]
pub fn test_if(flags: u16) -> bool { test_flag(flags, IF) }
#[inline]
pub fn test_df(flags: u16) -> bool { test_flag(flags, DF) }

#[inline]
pub fn set_df(flags: &mut u16) { *flags |= DF; }
#[inline]
pub fn clear_df(flags: &mut u16) { *flags &= !DF; }
#[inline]
pub fn set_if(flags: &mut u16) { *flags |= IF; }
#[inline]
pub fn clear_if(flags: &mut u16) { *flags &= !IF; }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_flags_preserves_if_df() {
        // Имитация: IF=1, DF=1, остальные 0
        let mut initial = 0u16;
        set_if(&mut initial);
        set_df(&mut initial);
        assert_eq!(initial & IF, IF);
        assert_eq!(initial & DF, DF);

        // Выполняем ADD, который должен обновить CF/ZF, но сохранить IF/DF
        let new_flags = compute_flags_u8(initial, 0x00, true, false, false);
        
        // Проверяем, что IF и DF остались включенными
        assert!(test_if(new_flags), "IF должен сохраниться после арифметики");
        assert!(test_df(new_flags), "DF должен сохраниться после арифметики");
        assert!(test_cf(new_flags), "CF должен установиться");
        assert!(test_zf(new_flags), "ZF должен установиться");
    }
}