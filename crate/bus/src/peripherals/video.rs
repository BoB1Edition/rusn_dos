// Ver: 14 File: crate/bus/src/peripherals/video.rs

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
    pub text_buffer: [u16; 80 * 25],
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
    pub cursor_start: u8,
    pub cursor_end: u8,
    pub planes: Box<[[u8; 0x10000]; 4]>,
    pub seq_index: u8,
    pub seq_regs: [u8; 5],
    pub grc_index: u8,
    pub grc_regs: [u8; 9],
    latches: Cell<[u8; 4]>,
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
        p[i] = STD[i - 16]; // 16..31 — зеркало 0..15, как грузит BIOS
    }
    for i in 32..256 {
        // аппроксимация BIOS-рампы для остальных индексов
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
    pub fn new() -> Self {
        Self {
            mode: VideoMode::Text80x25,
            text_buffer: [0x0720; 80 * 25],
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
            cursor_start: 6,
            cursor_end: 7,
            planes: Box::new([[0u8; 0x10000]; 4]),
            seq_index: 0,
            seq_regs: [0x01, 0x0F, 0x0F, 0x00, 0x02],
            grc_index: 0,
            grc_regs: [0, 0, 0, 0, 0, 0, 0, 0, 0xFF],
            latches: Cell::new([0u8; 4]),
        }
    }

    const FRAME_NS: u128 = 16_666_667; // 60 Гц
    const VBLANK_NS: u128 = 1_400_000; // окно вертикального обратного хода
    const HFRAME_NS: u128 = 63_500; // ~15.75 кГц
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

    pub fn set_mode(&mut self, mode: VideoMode) {
        self.mode = mode;
        match mode {
            VideoMode::Text80x25 => self.text_buffer = [0x0720; 80 * 25],
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
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.dirty = true;
        self.cursor_start = 6;
        self.cursor_end = 7;
        self.reset_dac();
    }

    pub fn set_mode_u8(&mut self, mode: u8) -> bool {
        match mode & 0x7F {
            // бит 7 = «не очищать память», игнорируем очистку всё равно
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
        // Копируем строки 1..24 на место 0..23
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
                /* Carriage Return: возврат в начало строки */
                self.cursor_col = 0;
            }
            _ => {
                /* Печатный символ */
                let idx = self.cursor_row as usize * 80 + self.cursor_col as usize;
                self.text_buffer[idx] = ((attr as u16) << 8) | (ch as u16);

                // Двигаем курсор вправо
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
        self.dirty = true;
    }

    const TEXT_BASE: u32 = 0x18000;
    const TEXT_SIZE: u32 = 80 * 25 * 2; // 4000 байт

    pub fn read_mmio(&self, offset: u32) -> u8 {
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

// Реализуем Peripheral для будущего DeviceMapper
impl Peripheral for VideoMemory {
    fn mem_read(&mut self, addr: u32) -> u8 {
        self.read_mmio(addr)
    }

    fn mem_write(&mut self, addr: u32, val: u8) {
        self.write_mmio(addr, val);
    }

    fn port_read(&mut self, port: u16) -> u8 {
        match port {
            0x3DA => self.input_status_1(),
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
        let rowbytes = w / 8;
        let mut out = vec![0u32; w * h];
        for y in 0..h {
            for xb in 0..rowbytes {
                let off = y * rowbytes + xb;
                let b = [
                    self.planes[0][off],
                    self.planes[1][off],
                    self.planes[2][off],
                    self.planes[3][off],
                ];
                for bit in 0..8usize {
                    let s = 7 - bit;
                    let idx = (((b[0] >> s) & 1)
                        | (((b[1] >> s) & 1) << 1)
                        | (((b[2] >> s) & 1) << 2)
                        | (((b[3] >> s) & 1) << 3)) as usize;
                    let c = palette[idx];
                    out[y * w + xb * 8 + bit] =
                        (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32;
                }
            }
        }
        out
    }
}
