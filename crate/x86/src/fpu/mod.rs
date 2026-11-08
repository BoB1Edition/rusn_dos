// Ver: 7 File: crate/x86/src/fpu/mod.rs
//! x87 FPU: программная эмуляция 80-битного расширенного формата.

pub mod f80;
pub mod arith;
pub mod convert;
pub mod state;
pub mod esc;
pub mod control;

pub use f80::{F80, F80Class};
pub use arith::{
    add, sub, mul, div, sqrt, neg, cmp, ftst,
    RoundingMode, ExceptionFlags, CmpResult,
};
pub use convert::{
    from_i16, from_i32, from_i64,
    to_i16, to_i32, to_i64,
    from_f32, from_f32_bits,
    from_f64, from_f64_bits,
    to_f32, to_f32_bits,
    to_f64, to_f64_bits,
    from_bcd, to_bcd, BCD_DIGITS,
};
pub use state::{FpuState, Tag};
pub use esc::{execute_esc, CR0_MP, CR0_EM, CR0_TS};