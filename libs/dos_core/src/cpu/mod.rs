//! Модуль центрального процессора (CPU)
//! 
//! Содержит компоненты для обработки флагов, префиксов и выполнения инструкций.

pub mod flags;
pub mod executor;


// Экспортируем все функции флагов для удобного использования
pub use flags::{
    compute_flags_u8, compute_flags_u16, compute_flags_u32,
    compute_logical_flags_u8, compute_logical_flags_u16, compute_logical_flags_u32,
    test_flag, set_flag,
    test_cf, test_zf, test_sf, test_of,
    clear_df, set_if, clear_if,
    CF, PF, AF, ZF, SF, OF,
};