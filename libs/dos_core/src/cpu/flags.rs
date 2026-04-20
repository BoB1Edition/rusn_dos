// Ver: 1
pub const CF: u16 = 1 << 0;
pub const PF: u16 = 1 << 2;
pub const AF: u16 = 1 << 4;
pub const ZF: u16 = 1 << 6;
pub const SF: u16 = 1 << 7;
pub const OF: u16 = 1 << 11;
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

#[inline]
pub fn compute_flags_u8(result: u8, cf: bool, of: bool, af: bool) -> u16 {
    compute_flags_impl(result as u32, 8, cf, of, af)
}

#[inline]
pub fn compute_flags_u16(result: u16, cf: bool, of: bool, af: bool) -> u16 {
    compute_flags_impl(result as u32, 16, cf, of, af)
}

#[inline]
pub fn compute_flags_u32(result: u32, cf: bool, of: bool, af: bool) -> u16 {
    compute_flags_impl(result, 32, cf, of, af)
}

#[inline]
fn compute_flags_impl(value: u32, width_bits: u32, cf: bool, of: bool, af: bool) -> u16 {
    let mut flags = 0u16;
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
pub fn compute_logical_flags_u8(result: u8) -> u16 {
    compute_flags_u8(result, false, false, false)
}

#[inline]
pub fn compute_logical_flags_u16(result: u16) -> u16 {
    compute_flags_u16(result, false, false, false)
}

#[inline]
pub fn compute_logical_flags_u32(result: u32) -> u16 {
    compute_flags_u32(result, false, false, false)
}

#[inline]
pub fn test_flag(flags: u16, bit_mask: u16) -> bool {
    (flags & bit_mask) != 0
}

#[inline]
pub fn set_flag(flags: &mut u16, bit_mask: u16, value: bool) {
    if value {
        *flags |= bit_mask;
    } else {
        *flags &= !bit_mask;
    }
}

#[inline]
pub fn test_cf(flags: u16) -> bool {
    test_flag(flags, CF)
}

#[inline]
pub fn test_zf(flags: u16) -> bool {
    test_flag(flags, ZF)
}

#[inline]
pub fn test_sf(flags: u16) -> bool {
    test_flag(flags, SF)
}

#[inline]
pub fn test_of(flags: u16) -> bool {
    test_flag(flags, OF)
}

#[inline]
pub fn test_pf(flags: u16) -> bool {
    test_flag(flags, PF)
}

#[inline]
pub fn clear_df(flags: &mut u16) {
    *flags &= !(DF);
}

#[inline]
pub fn set_if(flags: &mut u16) {
    *flags |= IF;
}

#[inline]
pub fn clear_if(flags: &mut u16) {
    *flags &= !(IF);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_flags_add_8bit() {
        let flags = compute_flags_u8(0x00, true, false, true);
        assert!(test_cf(flags));
        assert!(test_zf(flags));
        assert!(!test_sf(flags));
        assert!(test_pf(flags));
        assert!(!test_of(flags));
    }

    #[test]
    fn test_compute_flags_overflow() {
        let flags = compute_flags_u8(0x80, false, true, false);
        assert!(!test_cf(flags));
        assert!(!test_zf(flags));
        assert!(test_sf(flags));
        assert!(!test_pf(flags));
        assert!(test_of(flags));
    }

    #[test]
    fn test_logical_flags() {
        let flags = compute_logical_flags_u8(0x0F);
        assert!(!test_cf(flags));
        assert!(!test_zf(flags));
        assert!(!test_sf(flags));
        assert!(test_pf(flags));
        assert!(!test_of(flags));
    }
}