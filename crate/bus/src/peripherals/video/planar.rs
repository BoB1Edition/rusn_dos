use crate::peripherals::fonts_vga8x16::VGA_FONT_8X16;

use super::VideoMemory;

impl VideoMemory {
    pub fn planar_read(&self, offset: u32) -> u8 {
        let off = (offset as usize) & 0xFFFF;
        let l = [
            self.planes[0][off],
            self.planes[1][off],
            self.planes[2][off],
            self.planes[3][off],
        ];
        self.latches.set(l);
        let read_mode = (self.grc_regs[5] >> 3) & 1;
        if read_mode == 0 {
            l[(self.grc_regs[4] & 0x03) as usize]
        } else {
            let cmp = self.grc_regs[2] & 0x0F;
            let dc = self.grc_regs[7] & 0x0F;
            let mut res = 0u8;
            for bit in 0..8u8 {
                let s = 7 - bit;
                let mut color = 0u8;
                for p in 0..4u8 {
                    color |= ((l[p as usize] >> s) & 1) << p;
                }
                if (color ^ cmp) & dc == 0 {
                    res |= 1 << bit;
                }
            }
            res
        }
    }

    pub fn planar_write(&mut self, offset: u32, value: u8) {
        let off = (offset as usize) & 0xFFFF;
        let map_mask = self.seq_regs[2] & 0x0F;
        let write_mode = self.grc_regs[5] & 0x03;
        let bit_mask = self.grc_regs[8];
        let rotate = (self.grc_regs[3] & 0x07) as u32;
        let alu = (self.grc_regs[3] >> 3) & 0x03;
        let sr = self.grc_regs[0] & 0x0F;
        let sre = self.grc_regs[1] & 0x0F;
        let rot = if rotate == 0 {
            value
        } else {
            (((value as u32) >> rotate) | ((value as u32) << (8 - rotate))) as u8 & 0xFF
        };
        let l = self.latches.get();
        for p in 0..4usize {
            if (map_mask >> p) & 1 == 0 && write_mode != 1 {
                continue;
            }
            let latch = l[p];
            let new_byte = match write_mode {
                0 => {
                    let src = if (sre >> p) & 1 == 1 {
                        expand((sr >> p) & 1)
                    } else {
                        rot
                    };
                    let op = match alu {
                        0 => src,
                        1 => src & latch,
                        2 => src | latch,
                        _ => src ^ latch,
                    };
                    (op & bit_mask) | (latch & !bit_mask)
                }
                1 => latch,
                2 => {
                    let col = expand((value >> p) & 1);
                    (col & bit_mask) | (latch & !bit_mask)
                }
                _ => {
                    let mask = rot & bit_mask;
                    let col = expand((sr >> p) & 1);
                    (col & mask) | (latch & !mask)
                }
            };
            self.planes[p][off] = new_byte;
        }
    }

    pub fn render_planar(&self, w: usize, h: usize, palette: &[[u8; 3]; 256]) -> Vec<u32> {
        let line_bytes = match self.offset_bytes() {
            0 => w / 8,
            n => n,
        };
        let start = self.start_address();
        let cols = self.display_cols() * 8;
        let rows = self.display_rows(h);
        let bc = palette[self.border_dac()];
        let border_c = (bc[0] as u32) << 16 | (bc[1] as u32) << 8 | bc[2] as u32;
        let mut out = vec![0u32; w * h];
        for y in 0..h {
            for xb in 0..(w / 8) {
                let off = start + y * line_bytes + xb;
                let b = [
                    self.planes[0].get(off).copied().unwrap_or(0),
                    self.planes[1].get(off).copied().unwrap_or(0),
                    self.planes[2].get(off).copied().unwrap_or(0),
                    self.planes[3].get(off).copied().unwrap_or(0),
                ];
                for bit in 0..8usize {
                    let x = xb * 8 + bit;
                    out[y * w + x] = if x >= cols || y >= rows {
                        border_c
                    } else {
                        let s = 7 - bit;
                        let mut idx = (((b[0] >> s) & 1)
                            | (((b[1] >> s) & 1) << 1)
                            | (((b[2] >> s) & 1) << 2)
                            | (((b[3] >> s) & 1) << 3))
                            as usize;
                        idx &= self.plane_enable() as usize;
                        let c = palette[self.ac_dac_index_graphics(idx)];
                        (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32
                    };
                }
            }
        }
        self.apply_pan(out, w, h)
    }

    pub fn render_text(&self, palette: &[[u8; 3]; 256]) -> Vec<u32> {
        const WIDTH: usize = 640;
        const HEIGHT: usize = 400;
        let mut raw = vec![0u32; WIDTH * HEIGHT];
        let font = VGA_FONT_8X16;
        let line = self.text_line_cells();
        let start = self.start_address() / 2;
        let cur_cell = self.cursor_address() / 2;
        let cur_on = self.cursor_enabled();
        let (cs, ce) = (self.cursor_scan_start(), self.cursor_scan_end());
        let cols = self.display_cols();
        let rows = self.display_rows(25);
        let bc = palette[self.border_dac()];
        let border_c = (bc[0] as u32) << 16 | (bc[1] as u32) << 8 | bc[2] as u32;
        for row in 0..25usize {
            for col in 0..80usize {
                let base = row * 16 * WIDTH + col * 8;
                if col >= cols || row >= rows {
                    for y in 0..16usize {
                        let b = base + y * WIDTH;
                        raw[b..b + 8].fill(border_c);
                    }
                    continue;
                }
                let addr = start + row * line + col;
                let cell = self.text_buffer.get(addr).copied().unwrap_or(0x0720);
                let ch = (cell & 0xFF) as usize;
                let attr = ((cell >> 8) & 0xFF) as usize;
                let fg4 = attr & 0x0F;
                let bg4 = if self.blink_enabled() {
                    (attr >> 4) & 0x07
                } else {
                    (attr >> 4) & 0x0F
                };
                let fg = palette[self.ac_dac_index(fg4)];
                let bg = palette[self.ac_dac_index(bg4)];
                let fg_c = (fg[0] as u32) << 16 | (fg[1] as u32) << 8 | fg[2] as u32;
                let bg_c = (bg[0] as u32) << 16 | (bg[1] as u32) << 8 | bg[2] as u32;
                let blink = self.blink_enabled() && (attr & 0x80) != 0;
                let fg_visible = !blink || self.blink_on;
                let is_cur = cur_on && cs <= ce && addr == cur_cell;
                for y in 0..16usize {
                    let glyph_row = font[ch * 16 + y];
                    let mut colors = [bg_c; 8];
                    for x in 0..8usize {
                        if (glyph_row >> (7 - x)) & 1 == 1 && fg_visible {
                            colors[x] = fg_c;
                        }
                    }
                    if is_cur && y >= cs && y <= ce {
                        colors = [fg_c; 8];
                    }
                    let b = base + y * WIDTH;
                    raw[b..b + 8].copy_from_slice(&colors);
                }
            }
        }
        self.apply_pan(raw, WIDTH, HEIGHT)
    }

    #[inline]
    pub fn ac_dac_index(&self, idx4: usize) -> usize {
        let p = self.ac_regs[idx4 & 0x0F] as usize;
        let cs = self.ac_regs[0x14] as usize;
        (p & 0x3F) | ((cs & 0x03) << 6)
    }
    #[inline]
    pub fn ac_dac_index_graphics(&self, idx4: usize) -> usize {
        let p = self.ac_regs[idx4 & 0x0F] as usize;
        let cs = self.ac_regs[0x14] as usize;
        (p & 0x0F) | ((cs & 0x0C) << 2) | ((cs & 0x03) << 6)
    }
    #[inline]
    pub fn plane_enable(&self) -> u8 {
        self.ac_regs[0x12] & 0x0F
    }
    #[inline]
    pub fn blink_enabled(&self) -> bool {
        (self.ac_regs[0x10] & 0x08) != 0
    }
}

#[inline]
fn expand(bit: u8) -> u8 {
    if bit != 0 { 0xFF } else { 0x00 }
}
