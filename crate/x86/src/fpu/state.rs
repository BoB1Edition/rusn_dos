// Ver: 3 File: crate/x86/src/fpu/state.rs

use super::arith::{ExceptionFlags, RoundingMode};
use super::f80::{F80, F80Class};

pub const STAT_IE: u16 = 1 << 0;
pub const STAT_DE: u16 = 1 << 1;
pub const STAT_ZE: u16 = 1 << 2;
pub const STAT_OE: u16 = 1 << 3;
pub const STAT_UE: u16 = 1 << 4;
pub const STAT_PE: u16 = 1 << 5;
pub const STAT_SF: u16 = 1 << 6;
pub const STAT_ES: u16 = 1 << 7;
pub const STAT_C0: u16 = 1 << 8;
pub const STAT_C1: u16 = 1 << 9;
pub const STAT_C2: u16 = 1 << 10;
pub const STAT_C3: u16 = 1 << 14;

const TOP_SHIFT: u16 = 11;
const TOP_MASK: u16 = 0x3800;

const EXCEPT_MASK: u16 = STAT_IE | STAT_DE | STAT_ZE | STAT_OE | STAT_UE | STAT_PE;

const CC_MASK: u16 = STAT_C3 | STAT_C2 | STAT_C1 | STAT_C0;

pub const CTRL_IM: u16 = 1 << 0;
pub const CTRL_DM: u16 = 1 << 1;
pub const CTRL_ZM: u16 = 1 << 2;
pub const CTRL_OM: u16 = 1 << 3;
pub const CTRL_UM: u16 = 1 << 4;
pub const CTRL_PM: u16 = 1 << 5;

pub const CTRL_DEFAULT: u16 = 0x037F;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tag {
    Valid,
    Zero,
    Special,
    Empty,
}

impl Tag {
    #[inline]
    pub fn bits(self) -> u8 {
        match self {
            Tag::Valid => 0b00,
            Tag::Zero => 0b01,
            Tag::Special => 0b10,
            Tag::Empty => 0b11,
        }
    }

    #[inline]
    pub fn from_bits(b: u8) -> Self {
        match b & 3 {
            0b00 => Tag::Valid,
            0b01 => Tag::Zero,
            0b10 => Tag::Special,
            _ => Tag::Empty,
        }
    }

    pub fn for_value(f: F80) -> Self {
        match f.classify() {
            F80Class::Zero => Tag::Zero,
            F80Class::Normal => Tag::Valid,
            _ => Tag::Special,
        }
    }
}

pub struct FpuState {
    regs: [F80; 8],
    tags: u16,
    pub status: u16,
    pub control: u16,
    pub fip: u32,
    pub fcs: u16,
    pub fdp: u32,
    pub fds: u16,
    pub opcode: u16,
}

impl FpuState {
    pub fn new() -> Self {
        Self {
            regs: [F80::default(); 8],
            tags: 0xFFFF,
            status: 0,   
            control: CTRL_DEFAULT,
            fip: 0,
            fcs: 0,
            fdp: 0,
            fds: 0,
            opcode: 0,
        }
    }

    pub fn finit(&mut self) {
        *self = Self::new();
    }

    #[inline]
    pub fn top(&self) -> usize {
        ((self.status >> TOP_SHIFT) & 7) as usize
    }

    #[inline]
    fn set_top(&mut self, t: usize) {
        self.status = (self.status & !TOP_MASK) | (((t & 7) as u16) << TOP_SHIFT);
    }

    #[inline]
    pub fn phys(&self, i: usize) -> usize {
        (self.top() + i) & 7
    }

    #[inline]
    pub fn tag_of(&self, phys: usize) -> Tag {
        Tag::from_bits(((self.tags >> ((phys & 7) * 2)) & 3) as u8)
    }

    #[inline]
    pub fn set_tag(&mut self, phys: usize, tag: Tag) {
        let shift = (phys & 7) * 2;
        self.tags = (self.tags & !(3u16 << shift)) | ((tag.bits() as u16) << shift);
    }

    #[inline]
    pub fn st_tag(&self, i: usize) -> Tag {
        self.tag_of(self.phys(i))
    }

    #[inline]
    pub fn st(&self, i: usize) -> F80 {
        self.regs[self.phys(i)]
    }

    #[inline]
    pub fn peek(&self) -> F80 {
        self.st(0)
    }

    pub fn set_st(&mut self, i: usize, value: F80) {
        let p = self.phys(i);
        self.regs[p] = value;
        self.set_tag(p, Tag::for_value(value));
    }

    #[inline]
    pub fn raw(&self, phys: usize) -> F80 {
        self.regs[phys & 7]
    }

    #[inline]
    pub fn set_raw(&mut self, phys: usize, value: F80) {
        self.regs[phys & 7] = value;
    }

    #[inline]
    pub fn tag_word(&self) -> u16 {
        self.tags
    }

    #[inline]
    pub fn set_tag_word(&mut self, tags: u16) {
        self.tags = tags;
    }

    pub fn push(&mut self, value: F80) -> Result<(), ()> {
        let target = (self.top() + 7) & 7;

        if self.tag_of(target) != Tag::Empty {
            self.stack_fault();
            return Err(());
        }

        self.set_top(target);
        self.regs[target] = value;
        self.set_tag(target, Tag::for_value(value));
        Ok(())
    }

    pub fn pop(&mut self) -> Result<F80, ()> {
        let t = self.top();

        if self.tag_of(t) == Tag::Empty {
            self.stack_fault();
            return Err(());
        }

        let value = self.regs[t];
        self.set_tag(t, Tag::Empty);
        self.set_top((t + 1) & 7);
        Ok(value)
    }

    pub fn check_st(&mut self, i: usize) -> bool {
        if self.st_tag(i) == Tag::Empty {
            self.stack_fault();
            return false;
        }
        true
    }

    fn stack_fault(&mut self) {
        self.status |= STAT_SF;
        self.raise(ExceptionFlags {
            invalid: true,
            ..ExceptionFlags::default()
        });
    }

    pub fn raise(&mut self, ex: ExceptionFlags) {
        let mut raised = 0u16;
        if ex.invalid {
            raised |= STAT_IE;
        }
        if ex.denormal {
            raised |= STAT_DE;
        }
        if ex.div_zero {
            raised |= STAT_ZE;
        }
        if ex.overflow {
            raised |= STAT_OE;
        }
        if ex.underflow {
            raised |= STAT_UE;
        }
        if ex.precision {
            raised |= STAT_PE;
        }

        self.status |= raised;

        let unmasked = raised & !self.control & EXCEPT_MASK;
        if unmasked != 0 {
            self.status |= STAT_ES;
        }
    }

    pub fn fnclex(&mut self) {
        self.status &= !(EXCEPT_MASK | STAT_ES | STAT_SF);
    }

    pub fn set_cc(&mut self, c3: bool, c2: bool, c1: bool, c0: bool) {
        self.status &= !CC_MASK;
        if c3 {
            self.status |= STAT_C3;
        }
        if c2 {
            self.status |= STAT_C2;
        }
        if c1 {
            self.status |= STAT_C1;
        }
        if c0 {
            self.status |= STAT_C0;
        }
    }

    pub fn set_cc_from_cmp(&mut self, res: super::arith::CmpResult) {
        let (c3, c2, c1, c0) = res.condition_codes();
        self.set_cc(c3, c2, c1, c0);
    }

    #[inline]
    pub fn rounding_mode(&self) -> RoundingMode {
        RoundingMode::from_control(self.control)
    }

    #[inline]
    pub fn decstp(&mut self) {
        let t = self.top();
        self.set_top((t + 7) & 7);
    }

    #[inline]
    pub fn fincstp(&mut self) {
        let t = self.top();
        self.set_top((t + 1) & 7);
    }
}

impl Default for FpuState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one() -> F80 {
        F80::one(false)
    }

    fn two() -> F80 {
        F80::from_parts(false, 16384, 1u64 << 63)
    }

    #[test]
    fn test_finit_defaults() {
        let s = FpuState::new();
        assert_eq!(s.top(), 0);
        assert_eq!(s.status, 0);
        assert_eq!(s.control, CTRL_DEFAULT);
        assert_eq!(s.tag_word(), 0xFFFF);
        for i in 0..8 {
            assert_eq!(s.tag_of(i), Tag::Empty);
        }
    }

    #[test]
    fn test_push_pop_basic() {
        let mut s = FpuState::new();
        s.push(one()).unwrap();
        assert_eq!(s.peek(), one());
        assert_eq!(s.pop().unwrap(), one());
        assert_eq!(s.tag_of(s.top()), Tag::Empty);
    }

    #[test]
    fn test_top_rotation() {
        let mut s = FpuState::new();
        s.push(one()).unwrap();
        assert_eq!(s.top(), 7);
        s.push(two()).unwrap();
        assert_eq!(s.top(), 6);
        s.push(one()).unwrap();
        assert_eq!(s.top(), 5);
        assert_eq!(s.st(0), one());
        assert_eq!(s.st(1), two());
        assert_eq!(s.st(2), one());
    }

    #[test]
    fn test_st_mapping() {
        let mut s = FpuState::new();
        for i in 0..8u64 {
            s.push(F80::from_parts(false, 16383, 1u64 << 63 | i))
                .unwrap();
        }
        assert_eq!(s.top(), 0);
        assert_eq!(s.st(0).mant & 7, 7);
        assert_eq!(s.st(7).mant & 7, 0);
    }

    #[test]
    fn test_stack_overflow() {
        let mut s = FpuState::new();
        for _ in 0..8 {
            assert!(s.push(one()).is_ok());
        }
        let before = s.status;
        assert!(s.push(one()).is_err());
        assert_ne!(s.status & STAT_SF, 0, "SF должен быть поднят");
        assert_ne!(s.status & STAT_IE, 0, "IE должен быть поднят");
        assert_eq!(s.top(), 0);
        let _ = before;
    }

    #[test]
    fn test_stack_underflow() {
        let mut s = FpuState::new();
        let before_top = s.top();
        assert!(s.pop().is_err());
        assert_ne!(s.status & STAT_SF, 0);
        assert_ne!(s.status & STAT_IE, 0);
        assert_eq!(s.top(), before_top, "TOP не должен измениться");
    }

    #[test]
    fn test_check_st_empty() {
        let mut s = FpuState::new();
        assert!(!s.check_st(0));
        assert_ne!(s.status & STAT_SF, 0);
        assert_ne!(s.status & STAT_IE, 0);
    }

    #[test]
    fn test_tag_for_value() {
        assert_eq!(Tag::for_value(F80::zero(false)), Tag::Zero);
        assert_eq!(Tag::for_value(F80::zero(true)), Tag::Zero);
        assert_eq!(Tag::for_value(one()), Tag::Valid);
        assert_eq!(Tag::for_value(two()), Tag::Valid);
        assert_eq!(Tag::for_value(F80::infinity(false)), Tag::Special);
        assert_eq!(Tag::for_value(F80::qnan()), Tag::Special);
        assert_eq!(Tag::for_value(F80::snan()), Tag::Special);
        assert_eq!(Tag::for_value(F80::from_parts(false, 0, 1)), Tag::Special);
        assert_eq!(
            Tag::for_value(F80::from_parts(false, 0, (1u64 << 63) | 1)),
            Tag::Special
        );
        assert_eq!(Tag::for_value(F80::from_parts(false, 100, 1)), Tag::Special);
    }

    #[test]
    fn test_set_st_updates_tag() {
        let mut s = FpuState::new();
        s.push(one()).unwrap();
        assert_eq!(s.st_tag(0), Tag::Valid);
        s.set_st(0, F80::zero(false));
        assert_eq!(s.st_tag(0), Tag::Zero);
        s.set_st(0, F80::qnan());
        assert_eq!(s.st_tag(0), Tag::Special);
    }

    #[test]
    fn test_tags_packing() {
        let mut s = FpuState::new();
        assert_eq!(s.tag_word(), 0xFFFF);
        s.set_tag(0, Tag::Valid);
        assert_eq!(s.tag_of(0), Tag::Valid);
        assert_eq!(s.tag_word() & 3, 0b00);
        s.set_tag(7, Tag::Zero);
        assert_eq!(s.tag_of(7), Tag::Zero);
        assert_eq!((s.tag_word() >> 14) & 3, 0b01);
        assert_eq!(s.tag_of(3), Tag::Empty);
    }

    #[test]
    fn test_condition_codes() {
        let mut s = FpuState::new();
        s.set_cc(true, false, false, true); 
        assert_eq!(s.status & CC_MASK, STAT_C3 | STAT_C0);
        s.set_cc(false, false, false, false);
        assert_eq!(s.status & CC_MASK, 0);
        s.set_cc(true, false, false, false);
        assert_eq!(s.status & CC_MASK, STAT_C3);
        s.set_cc(false, false, false, true);
        assert_eq!(s.status & CC_MASK, STAT_C0);
    }

    #[test]
    fn test_cc_does_not_disturb_top() {
        let mut s = FpuState::new();
        s.push(one()).unwrap();
        s.push(one()).unwrap();
        let top_before = s.top();
        s.set_cc(true, true, true, true);
        assert_eq!(s.top(), top_before, "set_cc не должен трогать TOP");
        s.set_cc(false, false, false, false);
        assert_eq!(s.top(), top_before);
    }

    #[test]
    fn test_top_does_not_disturb_cc() {
        let mut s = FpuState::new();
        s.set_cc(true, false, false, true);
        let cc_before = s.status & CC_MASK;
        s.push(one()).unwrap();
        assert_eq!(
            s.status & CC_MASK,
            cc_before,
            "push не должен трогать коды условий"
        );
    }

    #[test]
    fn test_raise_masked_no_es() {
        let mut s = FpuState::new();
        assert_eq!(s.control, CTRL_DEFAULT);
        s.raise(ExceptionFlags {
            invalid: true,
            precision: true,
            ..ExceptionFlags::default()
        });
        assert_eq!(s.status & EXCEPT_MASK, STAT_IE | STAT_PE);
        assert_eq!(s.status & STAT_ES, 0);
    }

    #[test]
    fn test_raise_unmasked_sets_es() {
        let mut s = FpuState::new();
        s.control = CTRL_DEFAULT & !CTRL_IM;
        s.raise(ExceptionFlags {
            invalid: true,
            precision: true,
            ..ExceptionFlags::default()
        });
        assert_eq!(s.status & EXCEPT_MASK, STAT_IE | STAT_PE);
        assert_ne!(s.status & STAT_ES, 0, "ES должен быть поднят");
    }

    #[test]
    fn test_raise_all_unmasked() {
        let mut s = FpuState::new();
        s.control = 0;
        s.raise(ExceptionFlags {
            invalid: true,
            denormal: true,
            div_zero: true,
            overflow: true,
            underflow: true,
            precision: true,
        });
        assert_eq!(s.status & EXCEPT_MASK, EXCEPT_MASK);
        assert_ne!(s.status & STAT_ES, 0);
    }

    #[test]
    fn test_fnclex_clears() {
        let mut s = FpuState::new();
        s.raise(ExceptionFlags {
            invalid: true,
            ..ExceptionFlags::default()
        });
        s.status |= STAT_SF;
        s.set_cc(true, false, false, true);
        let cc_before = s.status & CC_MASK;

        s.fnclex();

        assert_eq!(s.status & EXCEPT_MASK, 0);
        assert_eq!(s.status & STAT_ES, 0);
        assert_eq!(s.status & STAT_SF, 0);
        assert_eq!(
            s.status & CC_MASK,
            cc_before,
            "FCLEX не должен трогать коды условий"
        );
    }

    #[test]
    fn test_rounding_mode_decode() {
        let mut s = FpuState::new();
        assert_eq!(s.rounding_mode(), RoundingMode::NearestEven);

        s.control = CTRL_DEFAULT | 0x0400;
        assert_eq!(s.rounding_mode(), RoundingMode::Down);

        s.control = CTRL_DEFAULT | 0x0800;
        assert_eq!(s.rounding_mode(), RoundingMode::Up);

        s.control = CTRL_DEFAULT | 0x0C00;
        assert_eq!(s.rounding_mode(), RoundingMode::TowardZero);
    }

    #[test]
    fn test_finit_resets_everything() {
        let mut s = FpuState::new();
        s.push(one()).unwrap();
        s.push(two()).unwrap();
        s.set_cc(true, true, true, true);
        s.raise(ExceptionFlags {
            invalid: true,
            ..ExceptionFlags::default()
        });
        s.control = 0;

        s.finit();

        assert_eq!(s.top(), 0);
        assert_eq!(s.status, 0);
        assert_eq!(s.control, CTRL_DEFAULT);
        assert_eq!(s.tag_word(), 0xFFFF);
    }
}
