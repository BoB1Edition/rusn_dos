use super::{VideoMemory, VbeMode, VideoMode};

impl VideoMemory {
    pub fn vbe_info_block(&self) -> [u8; 256] {
        let mut b = [0u8; 256];
        b[0..4].copy_from_slice(b"VESA");
        b[4..6].copy_from_slice(&0x0200u16.to_le_bytes());
        b[6..10].copy_from_slice(&Self::VBE_OEM_PTR.to_le_bytes());
        b[10..14].copy_from_slice(&0x0000_0001u32.to_le_bytes());
        b[14..18].copy_from_slice(&Self::VBE_MODE_LIST_PTR.to_le_bytes());
        b[18..20].copy_from_slice(&12u16.to_le_bytes());
        b
    }

    pub fn vbe_mode_info_block(&self, mode: VbeMode) -> [u8; 256] {
        let mut block = [0u8; 256];
        let (w, h) = mode.dims();
        let bpp = mode.bpp();
        let bpl = mode.bytes_per_line();

        block[0..2].copy_from_slice(&0x009Bu16.to_le_bytes());
        block[2] = 0x07;
        block[3] = 0x00;
        block[4..6].copy_from_slice(&64u16.to_le_bytes());
        block[6..8].copy_from_slice(&64u16.to_le_bytes());
        block[8..10].copy_from_slice(&0xA000u16.to_le_bytes());
        block[10..12].copy_from_slice(&0u16.to_le_bytes());
        block[16..18].copy_from_slice(&(bpl as u16).to_le_bytes());
        block[18..20].copy_from_slice(&(w as u16).to_le_bytes());
        block[20..22].copy_from_slice(&(h as u16).to_le_bytes());
        block[22] = 8;
        block[23] = 16;
        block[24] = 1;
        block[25] = bpp;
        block[26] = 1;
        block[27] = if mode.is_direct_color() { 6 } else { 4 };
        block[28] = 0;
        block[29] = 0;

        if mode.is_direct_color() {
            block[31] = 8;
            block[32] = 16;
            block[33] = 8;
            block[34] = 8;
            block[35] = 8;
            block[36] = 0;
            block[37] = 0;
            block[38] = 0;
        }

        block[40..44].copy_from_slice(&Self::VBE_LFB_BASE.to_le_bytes());

        block
    }

    pub fn window_read(&self, off: usize) -> u8 {
        self.vbe_lfb
            .get(self.vbe_bank * 65536 + off)
            .copied()
            .unwrap_or(0)
    }
    pub fn window_write(&mut self, off: usize, val: u8) {
        let addr = self.vbe_bank * 65536 + off;
        if addr < Self::VBE_LFB_SIZE {
            self.vbe_lfb[addr] = val;
            self.dirty = true;
        }
    }

    pub fn set_vbe_mode(&mut self, mode: VbeMode) {
        self.mode = VideoMode::Text80x25;
        self.vbe_mode = Some(mode);
        self.vbe_lfb.fill(0);
        self.vbe_bank = 0;
        self.dirty = true;
    }

    pub fn read_lfb(&self, offset: usize) -> u8 {
        self.vbe_lfb.get(offset).copied().unwrap_or(0)
    }

    pub fn write_lfb(&mut self, offset: usize, val: u8) {
        if offset < Self::VBE_LFB_SIZE {
            self.vbe_lfb[offset] = val;
            self.dirty = true;
        }
    }

    pub fn render_vbe(&self, mode: VbeMode, palette: &[[u8; 3]; 256]) -> Vec<u32> {
        let (w, h) = mode.dims();
        let mut out = vec![0u32; w * h];
        if mode.is_direct_color() {
            for y in 0..h {
                for x in 0..w {
                    let base = (y * w + x) * 3;
                    let b = self.vbe_lfb.get(base).copied().unwrap_or(0) as u32;
                    let g = self.vbe_lfb.get(base + 1).copied().unwrap_or(0) as u32;
                    let r = self.vbe_lfb.get(base + 2).copied().unwrap_or(0) as u32;
                    out[y * w + x] = (r << 16) | (g << 8) | b;
                }
            }
        } else {
            for y in 0..h {
                for x in 0..w {
                    let idx = self.vbe_lfb.get(y * w + x).copied().unwrap_or(0) as usize;
                    let c = palette[idx];
                    out[y * w + x] = ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | (c[2] as u32);
                }
            }
        }
        out
    }
}
