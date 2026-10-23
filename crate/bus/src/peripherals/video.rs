// Ver: 21 File: crate/bus/src/peripherals/video.rs

use std::cell::Cell;

use crate::peripheral::Peripheral;
use crate::peripherals::fonts_vga8x16::VGA_FONT_8X16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoMode {
    Text80x25,
    Mode13h,
    Mode0Dh,
    Mode0Eh,
    Mode10h,
    Mode12h,
}

impl VideoMode {
    /// (ширина, высота) для планарных 16-цветных режимов
    pub fn planar_dims(&self) -> Option<(usize, usize)> {
        match self {
            VideoMode::Mode0Dh => Some((320, 200)),
            VideoMode::Mode0Eh => Some((640, 200)),
            VideoMode::Mode10h => Some((640, 350)),
            VideoMode::Mode12h => Some((640, 480)),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct VideoMemory {
    pub mode: VideoMode,
    pub frame_buffer: [u8; 320 * 200],
    pub dirty: bool,
    pub cursor_row: u8,
    pub cursor_col: u8,
    retrace_epoch: std::time::Instant,
    pub attr_flip_flop: bool,
    pub dac: [[u8; 3]; 256],
    pub dac_pixel_mask: u8,
    dac_write_index: u8,
    dac_read_index: u8,
    dac_cycle: u8,
    dac_mode_read: bool,
    pub planes: Box<[[u8; 0x10000]; 4]>,
    pub seq_index: u8,
    pub seq_regs: [u8; 5],
    pub grc_index: u8,
    pub grc_regs: [u8; 9],
    latches: Cell<[u8; 4]>,
    pub crtc_index: u8,
    pub crtc_regs: [u8; 0x19],
    pub text_buffer: Box<[u16; 0x4000]>,
    pub ac_index: u8,
    pub ac_regs: [u8; 0x15],
    pub vbe_mode: Option<VbeMode>,
    pub vbe_lfb: Box<[u8; Self::VBE_LFB_SIZE]>,
    pub vbe_rom: Box<[u8; 0x200]>,
    pub vbe_bank: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VbeMode {
    Mode101_640x480x256,
    Mode103_800x600x256,
    Mode105_1024x768x256,
    Mode112_640x480x24bit,
}

impl VbeMode {
    pub fn from_number(n: u16) -> Option<Self> {
        match n {
            0x101 => Some(Self::Mode101_640x480x256),
            0x103 => Some(Self::Mode103_800x600x256),
            0x105 => Some(Self::Mode105_1024x768x256),
            0x112 => Some(Self::Mode112_640x480x24bit),
            _ => None,
        }
    }
    pub fn number(&self) -> u16 {
        match self {
            Self::Mode101_640x480x256 => 0x101,
            Self::Mode103_800x600x256 => 0x103,
            Self::Mode105_1024x768x256 => 0x105,
            Self::Mode112_640x480x24bit => 0x112,
        }
    }
    pub fn dims(&self) -> (usize, usize) {
        match self {
            Self::Mode101_640x480x256 | Self::Mode112_640x480x24bit => (640, 480),
            Self::Mode103_800x600x256 => (800, 600),
            Self::Mode105_1024x768x256 => (1024, 768),
        }
    }
    pub fn bpp(&self) -> u8 {
        match self {
            Self::Mode101_640x480x256 | Self::Mode103_800x600x256 | Self::Mode105_1024x768x256 => 8,
            Self::Mode112_640x480x24bit => 24,
        }
    }
    pub fn bytes_per_line(&self) -> usize {
        let (w, _) = self.dims();
        w * (self.bpp() as usize / 8)
    }
    pub fn is_direct_color(&self) -> bool {
        let b = matches!(self, Self::Mode112_640x480x24bit);
        b
    }
}

fn ac_defaults(mode: VideoMode) -> [u8; 0x15] {
    let mut r = [0u8; 0x15];
    for i in 0..16 {
        r[i] = i as u8; // identity, как грузит BIOS
    }
    r[0x10] = match mode {
        VideoMode::Text80x25 => 0x0C, // текст, blink включён (как реальный BIOS)
        VideoMode::Mode13h => 0x41,   // 256 цветов
        _ => 0x01,                    // 16-цветная графика
    };
    r[0x11] = 0x00; // overscan
    r[0x12] = 0x0F; // все плоскости включены
    r[0x13] = 0x00; // panning
    r[0x14] = 0x00; // color select
    r
}

pub fn default_dac_palette() -> [[u8; 3]; 256] {
    const STD: [[u8; 3]; 16] = [
        [0, 0, 0],
        [0, 0, 42],
        [0, 42, 0],
        [0, 42, 42],
        [42, 0, 0],
        [42, 0, 42],
        [42, 21, 0],
        [42, 42, 42],
        [21, 21, 21],
        [21, 21, 63],
        [21, 63, 21],
        [21, 63, 63],
        [63, 21, 21],
        [63, 21, 63],
        [63, 63, 21],
        [63, 63, 63],
    ];
    let mut p = [[0u8; 3]; 256];
    p[..16].copy_from_slice(&STD);
    for i in 16..32 {
        p[i] = STD[i - 16];
    }
    for i in 32..256 {
        p[i] = [
            ((i * 7) % 64) as u8,
            ((i * 5) % 64) as u8,
            ((i * 3) % 64) as u8,
        ];
    }
    p
}

#[inline]
fn six_to_eight(v: u8) -> u8 {
    (v << 2) | (v >> 4)
}

impl VideoMemory {
    pub const VBE_ROM_BASE: u32 = 0xFE000;
    pub const VBE_ROM_SIZE: usize = 0x200;
    pub const VBE_OEM_PTR: u32 = (0xF000 << 16) | 0xE000; // far pointer в блок
    pub const VBE_MODE_LIST_PTR: u32 = (0xF000 << 16) | 0xE100;
    pub const VBE_OEM_STRING: &[u8] = b"RustDOS VBE 2.0\0";
    pub const VBE_MODE_LIST: [u16; 5] = [0x101, 0x103, 0x105, 0x112, 0xFFFF];
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

    const FRAME_NS: u128 = 16_666_667;
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
        self.crtc_regs[0x13] = match self.mode {
            VideoMode::Text80x25 => 40,
            VideoMode::Mode0Dh => 10,
            VideoMode::Mode13h | VideoMode::Mode0Eh | VideoMode::Mode10h | VideoMode::Mode12h => 20,
        };
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
        // Очищаем последнюю (24-ю) строку пробелами
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
            0x07 => { /* Bell - игнорируем */ }
            0x08 => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            0x0A => {
                /* Line Feed: сдвигаем курсор вниз */
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
            return self.window_read(offset as usize); // read
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
            return; // write
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

    /// Палитра в 8 битах на канал для рендеринга
    pub fn palette_rgb8(&self) -> [[u8; 3]; 256] {
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

impl Peripheral for VideoMemory {
    fn mem_read(&mut self, addr: u32) -> u8 {
        self.read_mmio(addr)
    }

    fn mem_write(&mut self, addr: u32, val: u8) {
        self.write_mmio(addr, val);
    }

    fn port_read(&mut self, port: u16) -> u8 {
        match port {
            0x3D4 => self.crtc_index,
            0x3D5 => self
                .crtc_regs
                .get(self.crtc_index as usize)
                .copied()
                .unwrap_or(0),
            0x3DA => self.input_status_1(),
            0x3C0 => self.ac_index | ((self.attr_flip_flop as u8) << 5),
            0x3C1 => self
                .ac_regs
                .get(self.ac_index as usize)
                .copied()
                .unwrap_or(0),
            0x3C4 => self.seq_index,
            0x3C5 => self
                .seq_regs
                .get(self.seq_index as usize)
                .copied()
                .unwrap_or(0),
            0x3C6 => self.dac_pixel_mask,
            0x3C7 => {
                if self.dac_mode_read {
                    0x03
                } else {
                    0x00
                }
            }
            0x3C8 => self.dac_write_index,
            0x3C9 => {
                let v = self.dac[self.dac_read_index as usize][self.dac_cycle as usize];
                self.dac_cycle = (self.dac_cycle + 1) % 3;
                if self.dac_cycle == 0 {
                    self.dac_read_index = self.dac_read_index.wrapping_add(1);
                }
                v
            }
            0x3CE => self.grc_index,
            0x3CF => self
                .grc_regs
                .get(self.grc_index as usize)
                .copied()
                .unwrap_or(0),

            0x3C2 => 0x00,
            _ => {
                log::trace!("VGA port read {:#04x} not handled", port);
                0
            }
        }
    }

    fn port_write(&mut self, port: u16, val: u8) {
        match port {
            0x3D4 => self.crtc_index = val,
            0x3D5 => {
                let i = self.crtc_index as usize;
                if i < self.crtc_regs.len() {
                    self.crtc_regs[i] = val;
                    if i == 0x0E || i == 0x0F {
                        self.sync_cursor_from_crtc(); // ✅ железо дёргает BIOS-курсор
                    }
                    self.dirty = true;
                }
            }
            0x3C0 => {
                if !self.attr_flip_flop {
                    self.ac_index = val & 0x1F;
                } else {
                    let i = self.ac_index as usize;
                    if i < self.ac_regs.len() {
                        self.ac_regs[i] = val;
                        self.dirty = true;
                    }
                }
                self.attr_flip_flop = !self.attr_flip_flop; // ✅ чередование индекс/данные
            }
            0x3C1 => { /* read-only порт, записи игнорируем */ }
            0x3C6 => self.dac_pixel_mask = val,
            0x3C7 => {
                // индекс ЧТЕНИЯ + переход в read-состояние
                self.dac_read_index = val;
                self.dac_cycle = 0;
                self.dac_mode_read = true;
            }
            0x3C8 => {
                // индекс ЗАПИСИ + переход в write-состояние
                self.dac_write_index = val;
                self.dac_cycle = 0;
                self.dac_mode_read = false;
            }
            0x3C9 => {
                // поток R,G,B с автоинкрементом индекса
                self.dac[self.dac_write_index as usize][self.dac_cycle as usize] = val & 0x3F;
                self.dac_cycle = (self.dac_cycle + 1) % 3;
                if self.dac_cycle == 0 {
                    self.dac_write_index = self.dac_write_index.wrapping_add(1);
                }
                self.dirty = true; // смена палитры меняет картинку
            }
            0x3C4 => self.seq_index = val,
            0x3C5 => {
                let idx = self.seq_index as usize;
                if idx < self.seq_regs.len() {
                    self.seq_regs[idx] = val;
                }
            }
            0x3CE => self.grc_index = val,
            0x3CF => {
                let idx = self.grc_index as usize;
                if idx < self.grc_regs.len() {
                    self.grc_regs[idx] = val;
                }
            }
            0x3C2 => {}
            0x3DA => {}
            _ => {
                log::trace!("VGA port write {:#04x} = {:#04x} not handled", port, val);
            }
        }
    }
}

/// Стандартный VGA-шрифт 8x16 (4096 байт = 256 символов по 16 байт).
pub fn get_fonts_vga8x16() -> [u8; 4096] {
    VGA_FONT_8X16
}

pub fn upscale_framebuffer(fb: &[u8; 320 * 200], palette: &[[u8; 3]; 256]) -> Vec<u32> {
    const SRC_W: usize = 320;
    const SRC_H: usize = 200;
    const DST_W: usize = 1920; // 6× масштаб
    const DST_H: usize = 1200;

    let mut output = vec![0u32; DST_W * DST_H];

    for y in 0..SRC_H {
        for x in 0..SRC_W {
            let color_idx = fb[y * SRC_W + x] as usize;
            let color = palette[color_idx];
            let r = color[0] as u32;
            let g = color[1] as u32;
            let b = color[2] as u32;

            // Масштаб 6×6
            for dy in 0..6 {
                for dx in 0..6 {
                    let dst_x = x * 6 + dx;
                    let dst_y = y * 6 + dy;
                    if dst_x < DST_W && dst_y < DST_H {
                        output[dst_y * DST_W + dst_x] = (r << 16) | (g << 8) | b;
                    }
                }
            }
        }
    }
    output
}

pub fn render_text_to_pixels(
    text_buffer: &[u16; 80 * 25],
    font: &[u8; 4096],
    palette: &[[u8; 3]; 256],
) -> Vec<u32> {
    const WIDTH: usize = 640;
    const HEIGHT: usize = 400;
    let mut pixels = vec![0u32; WIDTH * HEIGHT];

    for row in 0..25 {
        for col in 0..80 {
            let cell = text_buffer[row * 80 + col];
            let ch = (cell & 0xFF) as usize;
            let attr = ((cell >> 8) & 0xFF) as usize;

            let fg = palette[attr & 0x0F];
            let bg = palette[(attr >> 4) & 0x0F];
            let fg_color = (fg[0] as u32) << 16 | (fg[1] as u32) << 8 | fg[2] as u32;
            let bg_color = (bg[0] as u32) << 16 | (bg[1] as u32) << 8 | bg[2] as u32;

            // Получаем 16 байт шрифта для текущего символа
            let glyph = &font[ch * 16..(ch * 16) + 16];

            // Рисуем символ 8x16 пикселей
            for y in 0..16 {
                let pixel_row = glyph[y];
                for x in 0..8 {
                    // Если бит установлен - рисуем цветом переднего плана, иначе - фона
                    let color = if (pixel_row >> (7 - x)) & 1 == 1 {
                        fg_color
                    } else {
                        bg_color
                    };

                    let px = col * 8 + x;
                    let py = row * 16 + y;
                    pixels[py * WIDTH + px] = color;
                }
            }
        }
    }
    pixels
}

pub fn scale_buffer(
    src: &[u32],
    src_width: usize,
    src_height: usize,
    dst_width: usize,
    dst_height: usize,
) -> Vec<u32> {
    let mut dst = vec![0u32; dst_width * dst_height];

    for y in 0..dst_height {
        for x in 0..dst_width {
            // Nearest-neighbor: находим соответствующий пиксель в исходном буфере
            let src_x = (x * src_width) / dst_width;
            let src_y = (y * src_height) / dst_height;
            dst[y * dst_width + x] = src[src_y * src_width + src_x];
        }
    }

    dst
}

#[inline]
fn expand(bit: u8) -> u8 {
    if bit != 0 { 0xFF } else { 0x00 }
}

impl VideoMemory {
    fn planar_read(&self, offset: u32) -> u8 {
        let off = (offset as usize) & 0xFFFF;
        let l = [
            self.planes[0][off],
            self.planes[1][off],
            self.planes[2][off],
            self.planes[3][off],
        ];
        self.latches.set(l); // ✅ побочный эффект: загрузка защёлок
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

    fn planar_write(&mut self, offset: u32, value: u8) {
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
                    let s = 7 - bit;
                    let mut idx = (((b[0] >> s) & 1)
                        | (((b[1] >> s) & 1) << 1)
                        | (((b[2] >> s) & 1) << 2)
                        | (((b[3] >> s) & 1) << 3)) as usize;
                    idx &= self.plane_enable() as usize;
                    let c = palette[self.ac_dac_index_graphics(idx)];
                    out[y * w + xb * 8 + bit] =
                        (c[0] as u32) << 16 | (c[1] as u32) << 8 | (c[2] as u32);
                }
            }
        }
        out
    }

    pub fn render_text(&self, palette: &[[u8; 3]; 256]) -> Vec<u32> {
        const WIDTH: usize = 640;
        const HEIGHT: usize = 400;
        let mut pixels = vec![0u32; WIDTH * HEIGHT];
        let font = VGA_FONT_8X16;
        let line = self.text_line_cells();
        let start = self.start_address() / 2;
        let cur_cell = self.cursor_address() / 2;
        let cur_on = self.cursor_enabled();
        let (cs, ce) = (self.cursor_scan_start(), self.cursor_scan_end());

        for row in 0..25usize {
            for col in 0..80usize {
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
                let is_cur = cur_on && cs <= ce && addr == cur_cell;
                for y in 0..16usize {
                    let glyph_row = font[ch * 16 + y];
                    let mut colors = [bg_c; 8];
                    for x in 0..8usize {
                        if (glyph_row >> (7 - x)) & 1 == 1 {
                            colors[x] = fg_c;
                        }
                    }
                    if is_cur && y >= cs && y <= ce {
                        colors = [fg_c; 8]; // сканлайны курсора
                    }
                    let base = (row * 16 + y) * WIDTH + col * 8;
                    pixels[base..base + 8].copy_from_slice(&colors);
                }
            }
        }
        pixels
    }

    pub fn render_mode13(&self, palette: &[[u8; 3]; 256]) -> Vec<u32> {
        const SRC_W: usize = 320;
        const SRC_H: usize = 200;
        const DST_W: usize = 1920;
        const DST_H: usize = 1200;
        let mut out = vec![0u32; DST_W * DST_H];
        let line_px = match self.offset_bytes() * 4 {
            0 => SRC_W,
            n => n,
        };
        let start_px = self.start_address() * 4;
        for y in 0..SRC_H {
            for x in 0..SRC_W {
                let idx = start_px + y * line_px + x;
                let ci = self.frame_buffer.get(idx).copied().unwrap_or(0) as usize;
                let c = palette[ci];
                let color = (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32;
                for dy in 0..6 {
                    for dx in 0..6 {
                        out[(y * 6 + dy) * DST_W + x * 6 + dx] = color;
                    }
                }
            }
        }
        out
    }
    #[inline]
    pub fn ac_dac_index(&self, idx4: usize) -> usize {
        let p = self.ac_regs[idx4 & 0x0F] as usize;
        let cs = self.ac_regs[0x14] as usize;
        (p & 0x3F) | ((cs & 0x03) << 6)
    }
    /// 16-цветная графика: биты 4-5 DAC берутся из Color Select 2-3
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

impl VideoMemory {
    pub const VBE_LFB_BASE: u32 = 0xE000_0000;
    pub const VBE_LFB_SIZE: usize = 1024 * 768;

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

        // ModeAttributes: бит 0 = mode supported, бит 1 = optional info,
        //                 бит 3 = color, бит 4 = graphics, бит 7 = linear FB
        block[0..2].copy_from_slice(&0x009Bu16.to_le_bytes());
        block[2] = 0x07; // WinA: exists, readable, writable
        block[3] = 0x00; // WinB: not supported
        block[4..6].copy_from_slice(&64u16.to_le_bytes()); // WinGranularity (KB)
        block[6..8].copy_from_slice(&64u16.to_le_bytes()); // WinSize (KB)
        block[8..10].copy_from_slice(&0xA000u16.to_le_bytes()); // WinASegment
        block[10..12].copy_from_slice(&0u16.to_le_bytes()); // WinBSegment
        block[16..18].copy_from_slice(&(bpl as u16).to_le_bytes());
        block[18..20].copy_from_slice(&(w as u16).to_le_bytes());
        block[20..22].copy_from_slice(&(h as u16).to_le_bytes());
        block[22] = 8; // XCharSize
        block[23] = 16; // YCharSize
        block[24] = 1; // NumberOfPlanes
        block[25] = bpp;
        block[26] = 1; // NumberOfBanks
        block[27] = if mode.is_direct_color() { 6 } else { 4 }; // MemoryModel: 4=packed, 6=direct
        block[28] = 0; // BankSize
        block[29] = 0; // NumberOfImagePages

        if mode.is_direct_color() {
            // Direct Color masks: 8:8:8
            block[31] = 8;
            block[32] = 16; // Red
            block[33] = 8;
            block[34] = 8; // Green
            block[35] = 8;
            block[36] = 0; // Blue
            block[37] = 0;
            block[38] = 0; // Reserved
        }

        // PhysBasePtr — LFB base
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
        self.mode = VideoMode::Text80x25; // сбрасываем «обычный» режим
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
            // 8bpp: индекс в DAC
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
