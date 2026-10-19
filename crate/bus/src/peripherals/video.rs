// Ver: 2 File: crate/bus/src/peripherals/video.rs

use crate::peripheral::Peripheral;
use crate::peripherals::fonts_vga8x16::VGA_FONT_8X16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoMode {
    Text80x25,
    Mode13h,
}

/// Стандартная VGA-палитра (16 базовых цветов + градиенты).
pub const VGA_COLORS: [u32; 16] = [
    0x000000, // 0: Black
    0x0000AA, // 1: Blue
    0x00AA00, // 2: Green
    0x00AAAA, // 3: Cyan
    0xAA0000, // 4: Red
    0xAA00AA, // 5: Magenta
    0xAA5500, // 6: Brown (Dark Yellow)
    0xAAAAAA, // 7: Light Gray
    0x555555, // 8: Dark Gray
    0x5555FF, // 9: Light Blue
    0x55FF55, // 10: Light Green
    0x55FFFF, // 11: Light Cyan
    0xFF5555, // 12: Light Red
    0xFF55FF, // 13: Light Magenta
    0xFFFF55, // 14: Yellow
    0xFFFFFF, // 15: White
];

#[derive(Debug)]
pub struct VideoMemory {
    pub mode: VideoMode,
    pub text_buffer: [u16; 80 * 25],
    pub frame_buffer: [u8; 320 * 200],
    pub dirty: bool,
    pub cursor_row: u8,
    pub cursor_col: u8,
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
        }
    }

    pub fn set_mode(&mut self, mode: VideoMode) {
        self.mode = mode;
        if mode == VideoMode::Text80x25 {
            self.text_buffer = [0x0720; 80 * 25];
        } else {
            self.frame_buffer = [0u8; 320 * 200];
        }
        self.dirty = true;
    }

    /// Установка режима по номеру BIOS (0x03 — текст, 0x13 — графика).
    pub fn set_mode_u8(&mut self, mode: u8) -> bool {
        match mode {
            0x03 => {
                self.set_mode(VideoMode::Text80x25);
                true
            }
            0x13 => {
                self.set_mode(VideoMode::Mode13h);
                true
            }
            _ => {
                log::warn!("Unsupported video mode {:02X}h", mode);
                false
            }
        }
    }

    pub fn read_mmio(&self, offset: u32) -> u8 {
        match self.mode {
            VideoMode::Text80x25 => {
                let idx = (offset / 2) as usize;
                if idx < self.text_buffer.len() {
                    let word = self.text_buffer[idx];
                    if offset % 2 == 0 {
                        (word & 0xFF) as u8
                    } else {
                        ((word >> 8) & 0xFF) as u8
                    }
                } else {
                    0
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
        }
    }

    pub fn write_mmio(&mut self, offset: u32, value: u8) {
        match self.mode {
            VideoMode::Text80x25 => {
                let idx = (offset / 2) as usize;
                if idx < self.text_buffer.len() {
                    let word = self.text_buffer[idx];
                    self.text_buffer[idx] = if offset % 2 == 0 {
                        (word & 0xFF00) | (value as u16)
                    } else {
                        (word & 0x00FF) | ((value as u16) << 8)
                    };
                    self.dirty = true;
                }
            }
            VideoMode::Mode13h => {
                let idx = offset as usize;
                if idx < self.frame_buffer.len() {
                    self.frame_buffer[idx] = value;
                    self.dirty = true;
                }
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

    // ✅ НОВОЕ: Логика телетайпа (INT 10h / AH=0Eh)
    pub fn write_tty(&mut self, ch: u8, attr: u8) {
        if self.mode != VideoMode::Text80x25 {
            log::debug!("INT 10h/0E in Mode13h: char {:#02x}", ch);
            return;
        }

        match ch {
            0x07 => { /* Bell - игнорируем */ }
            0x08 => {
                /* Backspace: сдвигаем курсор влево, но не стираем символ */
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
        // VGA-порты (заглушки для совместимости)
        match port {
            0x3C2 | 0x3C4 | 0x3C5 | 0x3CE | 0x3CF => 0x00,
            _ => 0,
        }
    }

    fn port_write(&mut self, port: u16, val: u8) {
        match port {
            0x3C2 | 0x3C4 | 0x3C5 | 0x3CE | 0x3CF => {
                log::trace!("VGA port write {:#04x} = {:#04x}", port, val);
            }
            _ => {}
        }
    }
}

/// Стандартный VGA-шрифт 8x16 (4096 байт = 256 символов по 16 байт).
pub fn get_fonts_vga8x16() -> [u8; 4096] {
    VGA_FONT_8X16
}

/// Загрузка стандартной VGA палитры (256 цветов).
pub fn load_vga_palette() -> [[u8; 3]; 256] {
    let mut palette = [[0u8; 3]; 256];

    // Цвет 0 = чёрный
    palette[0] = [0, 0, 0];
    // Цвет 1 = синий (для tasm_test3.asm)
    palette[1] = [0, 0, 170];
    // Цвет 2 = зелёный
    palette[2] = [0, 170, 0];
    // Цвет 3 = бирюзовый
    palette[3] = [0, 170, 170];
    // Цвет 4 = красный
    palette[4] = [170, 0, 0];
    // Цвет 5 = пурпурный
    palette[5] = [170, 0, 170];
    // Цвет 6 = коричневый/оранжевый
    palette[6] = [170, 85, 0];
    // Цвет 7 = светло-серый
    palette[7] = [170, 170, 170];
    // Цвет 8 = тёмно-серый
    palette[8] = [85, 85, 85];
    // Цвет 9 = ярко-синий
    palette[9] = [85, 85, 255];
    // Цвет 10 = ярко-зелёный
    palette[10] = [85, 255, 85];
    // Цвет 11 = ярко-бирюзовый
    palette[11] = [85, 255, 255];
    // Цвет 12 = ярко-красный
    palette[12] = [255, 85, 85];
    // Цвет 13 = ярко-пурпурный
    palette[13] = [255, 85, 255];
    // Цвет 14 = жёлтый
    palette[14] = [255, 255, 85];
    // Цвет 15 = белый
    palette[15] = [255, 255, 255];

    // Остальные цвета — градиенты (упрощённая реализация)
    for i in 16..256 {
        palette[i] = [
            ((i * 7) % 256) as u8,
            ((i * 5) % 256) as u8,
            ((i * 3) % 256) as u8,
        ];
    }

    palette
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

pub fn render_text_to_pixels(text_buffer: &[u16; 80 * 25], font: &[u8; 4096]) -> Vec<u32> {
    const WIDTH: usize = 640;
    const HEIGHT: usize = 400;
    let mut pixels = vec![0u32; WIDTH * HEIGHT];

    for row in 0..25 {
        for col in 0..80 {
            let cell = text_buffer[row * 80 + col];
            let ch = (cell & 0xFF) as usize;
            let attr = ((cell >> 8) & 0xFF) as usize;

            let fg_color = VGA_COLORS[attr & 0x0F];
            let bg_color = VGA_COLORS[(attr >> 4) & 0x0F];

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
