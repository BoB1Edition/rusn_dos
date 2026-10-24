use crate::peripherals::fonts_vga8x16::VGA_FONT_8X16;

use super::VideoMemory;
use super::types::VideoMode;

impl VideoMemory {
    pub fn render_mode13(&self, palette: &[[u8; 3]; 256]) -> Vec<u32> {
        const W: usize = 320;
        const H: usize = 200;
        let mut raw = vec![0u32; W * H];
        let line_px = match self.offset_bytes() * 4 {
            0 => W,
            n => n,
        };
        let start_px = self.start_address() * 4;
        let cols = self.display_cols() * 8;
        let rows = self.display_rows(H);
        let bc = palette[self.border_dac()];
        let border_c = (bc[0] as u32) << 16 | (bc[1] as u32) << 8 | bc[2] as u32;
        for y in 0..H {
            for x in 0..W {
                raw[y * W + x] = if x >= cols || y >= rows {
                    border_c
                } else {
                    let idx = start_px + y * line_px + x;
                    let ci = self.frame_buffer.get(idx).copied().unwrap_or(0) as usize;
                    let c = palette[ci];
                    (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32
                };
            }
        }
        self.apply_pan(raw, W, H)
    }

    pub fn blink_phase_on(&self) -> bool {
        let frames = self.retrace_epoch.elapsed().as_nanos() / Self::FRAME_NS;
        (frames % 32) < 16
    }

    pub fn blink_tick(&mut self) {
        let on = self.blink_phase_on();
        if on != self.blink_on {
            self.blink_on = on;
            if self.blink_enabled() && self.mode == VideoMode::Text80x25 {
                self.dirty = true;
            }
        }
    }

    #[inline]
    pub fn ac_dac_index_raw(&self, p: usize) -> usize {
        let cs = self.ac_regs[0x14] as usize;
        (p & 0x3F) | ((cs & 0x03) << 6)
    }
    #[inline]
    pub fn border_dac(&self) -> usize {
        self.ac_dac_index_raw(self.ac_regs[0x11] as usize)
    }
    pub fn display_cols(&self) -> usize {
        (self.crtc_regs[0x01] as usize + 1).min(80)
    }
    pub fn display_rows(&self, max: usize) -> usize {
        let v =
            (self.crtc_regs[0x12] as usize) | ((((self.crtc_regs[0x07] >> 1) & 1) as usize) << 8);
        (v + 1).min(max)
    }
    #[inline]
    pub fn pel_pan(&self) -> usize {
        (self.ac_regs[0x13] as usize) & 0x07
    }
    #[inline]
    pub fn preset_row_scan(&self) -> usize {
        (self.crtc_regs[0x08] as usize) & 0x1F
    }

    pub(crate) fn apply_pan(&self, src: Vec<u32>, w: usize, h: usize) -> Vec<u32> {
        let sx = self.pel_pan();
        let sy = self.preset_row_scan();
        if sx == 0 && sy == 0 {
            return src;
        }
        let mut out = vec![0u32; w * h];
        for y in 0..h {
            let syv = (y + sy) % h;
            for x in 0..w {
                out[y * w + x] = src[syv * w + ((x + sx) % w)];
            }
        }
        out
    }
}

pub fn get_fonts_vga8x16() -> [u8; 4096] {
    VGA_FONT_8X16
}

pub fn letterbox43(dw: usize, dh: usize) -> (usize, usize, usize, usize) {
    if dw * 3 == dh * 4 {
        (0, 0, dw, dh)
    } else if dw * 3 > dh * 4 {
        let rw = dh * 4 / 3;
        ((dw - rw) / 2, 0, rw, dh)
    } else {
        let rh = dw * 3 / 4;
        (0, (dh - rh) / 2, dw, rh)
    }
}

pub fn scale_into_rect(
    src: &[u32],
    sw: usize,
    sh: usize,
    dst: &mut [u32],
    dw: usize,
    dh: usize,
    ox: usize,
    oy: usize,
    rw: usize,
    rh: usize,
) {
    for px in dst.iter_mut() {
        *px = 0;
    }
    for y in 0..rh {
        let sy = (y * sh) / rh;
        let drow = (oy + y) * dw + ox;
        let srow = sy * sw;
        for x in 0..rw {
            let sx = (x * sw) / rw;
            dst[drow + x] = src[srow + sx];
        }
    }
}
