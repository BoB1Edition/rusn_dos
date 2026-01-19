// src/modrm.rs
#[derive(Debug, Clone, Copy)]
pub struct ModRm {
    pub mod_field: u8,
    pub reg_field: u8,
    pub rm_field: u8,
}

impl ModRm {
    #[inline]
    pub fn from_byte(byte: u8) -> Self {
        Self {
            mod_field: (byte >> 6) & 0x3,
            reg_field: (byte >> 3) & 0x7,
            rm_field: byte & 0x7,
        }
    }

    #[inline]
    pub fn is_register_mode(&self) -> bool {
        self.mod_field == 0b11
    }
}