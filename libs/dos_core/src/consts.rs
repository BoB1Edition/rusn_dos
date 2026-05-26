// Ver: 1 File: ./libs/dos_core/src/const.rs
/*
#[allow(non_upper_case_globals)]
pub const KiB: usize = 1024;
#[allow(non_upper_case_globals)]
pub const MiB: usize = 1024 * KiB;
pub const DOS_MEMORY_SIZE: usize = 1 * MiB;
pub const SEGMENT_SIZE: usize = 16;
*/

pub const DOS_MEMORY_SIZE: usize = 0xF00000;
/// Размер одного MCB в параграфах (всегда 1)
pub const MCB_SIZE_PARAGRAPHS: u16 = 1;
/// Сигнатуры MCB
pub const MCB_SIGNATURE_LAST: u8 = b'Z'; // последний блок в цепочке
pub const MCB_SIGNATURE_NON_LAST: u8 = b'M'; // не последний блок

