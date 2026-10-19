// Ver: 2 File: crate/dos/src/consts.rs

pub const DOS_MEMORY_SIZE: usize = 0xF00000;
pub const MCB_SIZE_PARAGRAPHS: u16 = 1;
pub const MCB_SIGNATURE_LAST: u8 = b'Z';
pub const MCB_SIGNATURE_NON_LAST: u8 = b'M';

pub const MAX_MCB_SEGMENT: u16 = 0xF000;