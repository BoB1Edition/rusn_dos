use std::cell::Cell;

use super::dac::{ac_defaults, default_dac_palette};
use super::types::{VideoMode, VbeMode};

#[derive(Debug)]
pub struct VideoMemory {
    pub mode: VideoMode,
    pub frame_buffer: [u8; 320 * 200],
    pub dirty: bool,
    pub cursor_row: u8,
    pub cursor_col: u8,
    pub(crate) retrace_epoch: std::time::Instant,
    pub attr_flip_flop: bool,
    pub dac: [[u8; 3]; 256],
    pub dac_pixel_mask: u8,
    pub(crate) dac_write_index: u8,
    pub(crate) dac_read_index: u8,
    pub(crate) dac_cycle: u8,
    pub(crate) dac_mode_read: bool,
    pub planes: Box<[[u8; 0x10000]; 4]>,
    pub seq_index: u8,
    pub seq_regs: [u8; 5],
    pub grc_index: u8,
    pub grc_regs: [u8; 9],
    pub(crate) latches: Cell<[u8; 4]>,
    pub crtc_index: u8,
    pub crtc_regs: [u8; 0x19],
    pub text_buffer: Box<[u16; 0x4000]>,
    pub ac_index: u8,
    pub ac_regs: [u8; 0x15],
    pub vbe_mode: Option<VbeMode>,
    pub vbe_lfb: Box<[u8; Self::VBE_LFB_SIZE]>,
    pub vbe_rom: Box<[u8; 0x200]>,
    pub vbe_bank: usize,
    pub blink_on: bool,
}

impl VideoMemory {
    pub const VBE_ROM_BASE: u32 = 0xFE000;
    pub const VBE_ROM_SIZE: usize = 0x200;
    pub const VBE_OEM_PTR: u32 = (0xF000 << 16) | 0xE000;
    pub const VBE_MODE_LIST_PTR: u32 = (0xF000 << 16) | 0xE100;
    pub const VBE_OEM_STRING: &[u8] = b"RustDOS VBE 2.0\0";
    pub const VBE_MODE_LIST: [u16; 5] = [0x101, 0x103, 0x105, 0x112, 0xFFFF];
    pub const VBE_LFB_BASE: u32 = 0xE000_0000;
    pub const VBE_LFB_SIZE: usize = 1024 * 768;

    pub fn new() -> Self {
        let mut v = Self {
            mode: VideoMode::Text80x25,
            text_buffer: Box::new([0x0720; 0x4000]),
            crtc_index: 0,
            crtc_regs: [0u8; 0x19],
            frame_buffer: [0u8; 320 * 200],
            dirty: false,
            cursor_row: 0,
            cursor_col: 0,
            retrace_epoch: std::time::Instant::now(),
            attr_flip_flop: false,
            dac: default_dac_palette(),
            dac_pixel_mask: 0xFF,
            dac_write_index: 0,
            dac_read_index: 0,
            dac_cycle: 0,
            dac_mode_read: false,
            planes: Box::new([[0u8; 0x10000]; 4]),
            seq_index: 0,
            seq_regs: [0x01, 0x0F, 0x0F, 0x00, 0x02],
            grc_index: 0,
            grc_regs: [0, 0, 0, 0, 0, 0, 0, 0, 0xFF],
            latches: Cell::new([0u8; 4]),
            ac_index: 0,
            ac_regs: ac_defaults(VideoMode::Text80x25),
            vbe_mode: None,
            vbe_lfb: Box::new([0u8; Self::VBE_LFB_SIZE]),
            vbe_rom: Box::new([0u8; 512]),
            vbe_bank: 0,
            blink_on: true,
        };
        v.crtc_defaults();
        v.vbe_rom[0..Self::VBE_OEM_STRING.len()].copy_from_slice(Self::VBE_OEM_STRING);
        for (i, &m) in Self::VBE_MODE_LIST.iter().enumerate() {
            let off = 0x100 + i * 2;
            v.vbe_rom[off] = (m & 0xFF) as u8;
            v.vbe_rom[off + 1] = (m >> 8) as u8;
        }
        v
    }

    pub(crate) const FRAME_NS: u128 = 16_666_667;
    const VBLANK_NS: u128 = 1_400_000;
    const HFRAME_NS: u128 = 63_500;
    const HBLANK_NS: u128 = 11_000;

    pub fn input_status_1(&mut self) -> u8 {
        self.attr_flip_flop = false;

        let t = self.retrace_epoch.elapsed().as_nanos();
        let vblank = t % Self::FRAME_NS >= Self::FRAME_NS - Self::VBLANK_NS;
        let hblank = t % Self::HFRAME_NS >= Self::HFRAME_NS - Self::HBLANK_NS;

        let mut status = 0u8;
        if hblank || vblank {
            status |= 0x01;
        }
        if vblank {
            status |= 0x08;
        }
        status
    }

    fn crtc_defaults(&mut self) {
        self.crtc_index = 0;
        self.crtc_regs = [0u8; 0x19];
        let (offset, cols, rows) = match self.mode {
            VideoMode::Text80x25 => (40, 80, 25),
            VideoMode::Mode0Dh => (10, 40, 200),
            VideoMode::Mode13h => (20, 40, 200),
            VideoMode::Mode0Eh => (20, 80, 200),
            VideoMode::Mode10h => (20, 80, 350),
            VideoMode::Mode12h => (20, 80, 480),
        };
        self.crtc_regs[0x13] = offset;
        self.crtc_regs[0x01] = (cols - 1) as u8;
        let ve = rows - 1;
        self.crtc_regs[0x12] = (ve & 0xFF) as u8;
        if ve & 0x100 != 0 {
            self.crtc_regs[0x07] |= 0x02;
        } else {
            self.crtc_regs[0x07] &= !0x02;
        }
        self.crtc_regs[0x0A] = 0x06;
        self.crtc_regs[0x0B] = 0x07;
    }

    pub fn set_mode(&mut self, mode: VideoMode) {
        self.mode = mode;
        match mode {
            VideoMode::Text80x25 => self.text_buffer = Box::new([0x0720; 0x4000]),
            VideoMode::Mode13h => self.frame_buffer = [0u8; 320 * 200],
            _ => {
                for p in self.planes.iter_mut() {
                    p.fill(0);
                }
                self.seq_index = 0;
                self.seq_regs = [0x01, 0x0F, 0x0F, 0x00, 0x02];
                self.grc_index = 0;
                self.grc_regs = [0, 0, 0, 0, 0, 0, 0, 0, 0xFF];
                self.latches.set([0u8; 4]);
            }
        }
        self.vbe_mode = None;
        self.crtc_defaults();
        self.ac_regs = ac_defaults(mode);
        self.ac_index = 0;
        self.attr_flip_flop = false;
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.dirty = true;
        self.reset_dac();
    }

    pub fn start_address(&self) -> usize {
        let lo = self.crtc_regs[0x0D] as usize;
        let hi = (self.crtc_regs[0x0C] as usize) << 8;
        let b16 = (((self.crtc_regs[0x07] >> 5) & 1) as usize) << 16;
        hi | lo | b16
    }

    pub fn offset_bytes(&self) -> usize {
        (self.crtc_regs[0x13] as usize) * 4
    }
    pub fn text_line_cells(&self) -> usize {
        match self.offset_bytes() / 2 {
            0 => 80,
            n => n,
        }
    }

    pub fn cursor_address(&self) -> usize {
        let lo = self.crtc_regs[0x0F] as usize;
        let hi = (self.crtc_regs[0x0E] as usize) << 8;
        let b16 = (((self.crtc_regs[0x07] >> 6) & 1) as usize) << 16;
        hi | lo | b16
    }
    pub fn cursor_enabled(&self) -> bool {
        (self.crtc_regs[0x0A] & 0x20) == 0
    }
    pub fn cursor_scan_start(&self) -> usize {
        (self.crtc_regs[0x0A] & 0x1F) as usize
    }
    pub fn cursor_scan_end(&self) -> usize {
        (self.crtc_regs[0x0B] & 0x1F) as usize
    }

    pub fn set_cursor_cell(&mut self, cell: usize) {
        let addr = cell * 2;
        self.crtc_regs[0x0E] = ((addr >> 8) & 0xFF) as u8;
        self.crtc_regs[0x0F] = (addr & 0xFF) as u8;
        if addr & 0x10000 != 0 {
            self.crtc_regs[0x07] |= 0x40;
        } else {
            self.crtc_regs[0x07] &= !0x40;
        }
    }

    pub fn sync_cursor_from_crtc(&mut self) {
        if self.mode != VideoMode::Text80x25 {
            return;
        }
        let line = self.text_line_cells();
        let cell = self.cursor_address() / 2;
        self.cursor_row = ((cell / line) as u8).min(24);
        self.cursor_col = ((cell % line) as u8).min(79);
    }

    pub fn set_mode_u8(&mut self, mode: u8) -> bool {
        match mode & 0x7F {
            0x03 => {
                self.set_mode(VideoMode::Text80x25);
                true
            }
            0x13 => {
                self.set_mode(VideoMode::Mode13h);
                true
            }
            0x0D => {
                self.set_mode(VideoMode::Mode0Dh);
                true
            }
            0x0E => {
                self.set_mode(VideoMode::Mode0Eh);
                true
            }
            0x10 => {
                self.set_mode(VideoMode::Mode10h);
                true
            }
            0x12 => {
                self.set_mode(VideoMode::Mode12h);
                true
            }
            _ => {
                log::warn!("Unsupported video mode {:02X}h", mode);
                false
            }
        }
    }

    pub fn scroll_up(&mut self) {
        for row in 0..24 {
            for col in 0..80 {
                self.text_buffer[row * 80 + col] = self.text_buffer[(row + 1) * 80 + col];
            }
        }
        for col in 0..80 {
            self.text_buffer[24 * 80 + col] = 0x0720;
        }
        self.dirty = true;
    }

    pub fn write_tty(&mut self, ch: u8, attr: u8) {
        if self.mode != VideoMode::Text80x25 {
            log::debug!("INT 10h/0E in Mode13h: char {:#02x}", ch);
            return;
        }

        match ch {
            0x07 => {}
            0x08 => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            0x0A => {
                self.cursor_row += 1;
                if self.cursor_row >= 25 {
                    self.scroll_up();
                    self.cursor_row = 24;
                }
            }
            0x0D => {
                self.cursor_col = 0;
            }
            _ => {
                let idx = self.cursor_row as usize * 80 + self.cursor_col as usize;
                self.text_buffer[idx] = ((attr as u16) << 8) | (ch as u16);
                self.cursor_col += 1;
                if self.cursor_col >= 80 {
                    self.cursor_col = 0;
                    self.cursor_row += 1;
                    if self.cursor_row >= 25 {
                        self.scroll_up();
                        self.cursor_row = 24;
                    }
                }
            }
        }
        let line = self.text_line_cells();
        self.set_cursor_cell(self.cursor_row as usize * line + self.cursor_col as usize);
        self.dirty = true;
    }

    const TEXT_BASE: u32 = 0x18000;
    const TEXT_SIZE: u32 = 0x8000;

    pub fn read_mmio(&self, offset: u32) -> u8 {
        if self.vbe_mode.is_some() && offset < 0x10000 {
            return self.window_read(offset as usize);
        }
        if self.mode.planar_dims().is_some() {
            return self.planar_read(offset);
        }
        match self.mode {
            VideoMode::Text80x25 => {
                if offset < Self::TEXT_BASE || offset >= Self::TEXT_BASE + Self::TEXT_SIZE {
                    return 0;
                }
                let local = offset - Self::TEXT_BASE;
                let idx = (local / 2) as usize;
                let word = self.text_buffer[idx];
                if local % 2 == 0 {
                    (word & 0xFF) as u8
                } else {
                    ((word >> 8) & 0xFF) as u8
                }
            }
            VideoMode::Mode13h => {
                let idx = offset as usize;
                if idx < self.frame_buffer.len() {
                    self.frame_buffer[idx]
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    pub fn write_mmio(&mut self, offset: u32, value: u8) {
        if self.vbe_mode.is_some() && offset < 0x10000 {
            self.window_write(offset as usize, value);
            return;
        }
        if self.mode.planar_dims().is_some() {
            self.planar_write(offset, value);
            self.dirty = true;
            return;
        }
        match self.mode {
            VideoMode::Text80x25 => {
                if offset < Self::TEXT_BASE || offset >= Self::TEXT_BASE + Self::TEXT_SIZE {
                    return;
                }
                let local = offset - Self::TEXT_BASE;
                let idx = (local / 2) as usize;
                let word = self.text_buffer[idx];
                self.text_buffer[idx] = if local % 2 == 0 {
                    (word & 0xFF00) | (value as u16)
                } else {
                    (word & 0x00FF) | ((value as u16) << 8)
                };
                self.dirty = true;
            }
            VideoMode::Mode13h => {
                let idx = offset as usize;
                if idx < self.frame_buffer.len() {
                    self.frame_buffer[idx] = value;
                    self.dirty = true;
                }
            }
            _ => {}
        }
    }
    pub fn reset_dac(&mut self) {
        self.dac = default_dac_palette();
        self.dac_pixel_mask = 0xFF;
        self.dac_write_index = 0;
        self.dac_read_index = 0;
        self.dac_cycle = 0;
        self.dac_mode_read = false;
        self.dirty = true;
    }

    pub fn palette_rgb8(&self) -> [[u8; 3]; 256] {
        use super::dac::six_to_eight;
        let mut out = [[0u8; 3]; 256];
        for (i, e) in self.dac.iter().enumerate() {
            out[i] = [six_to_eight(e[0]), six_to_eight(e[1]), six_to_eight(e[2])];
        }
        out
    }

    pub fn scroll_window(
        &mut self,
        top: usize,
        left: usize,
        bottom: usize,
        right: usize,
        lines: usize,
        attr: u8,
        up: bool,
    ) {
        let top = top.min(24);
        let bottom = bottom.min(24);
        let left = left.min(79);
        let right = right.min(79);
        if bottom < top || right < left {
            return;
        }
        let fill = ((attr as u16) << 8) | 0x20;
        let height = bottom - top + 1;

        if lines == 0 || lines >= height {
            for r in top..=bottom {
                for c in left..=right {
                    self.text_buffer[r * 80 + c] = fill;
                }
            }
        } else if up {
            for r in top..=bottom {
                for c in left..=right {
                    let src = if r + lines <= bottom {
                        self.text_buffer[(r + lines) * 80 + c]
                    } else {
                        fill
                    };
                    self.text_buffer[r * 80 + c] = src;
                }
            }
        } else {
            for r in (top..=bottom).rev() {
                for c in left..=right {
                    let src = if r >= top + lines {
                        self.text_buffer[(r - lines) * 80 + c]
                    } else {
                        fill
                    };
                    self.text_buffer[r * 80 + c] = src;
                }
            }
        }
        self.dirty = true;
    }

    pub fn write_cell_repeat(&mut self, ch: u8, attr: Option<u8>, count: usize) {
        let base = self.cursor_row as usize * 80 + self.cursor_col as usize;
        for i in 0..count {
            let pos = base + i;
            if pos >= 80 * 25 {
                break;
            }
            let old = self.text_buffer[pos];
            self.text_buffer[pos] = match attr {
                Some(a) => ((a as u16) << 8) | ch as u16,
                None => (old & 0xFF00) | ch as u16,
            };
        }
        self.dirty = true;
    }
}

impl Default for VideoMemory {
    fn default() -> Self {
        Self::new()
    }
}
