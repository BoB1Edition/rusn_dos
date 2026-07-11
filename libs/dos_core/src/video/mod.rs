// Ver: 3 File: ./libs/dos_core/src/video/mod.rs

use crate::video::fonts_vga8x16::VGA_FONT_8X16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoMode {
    Text80x25, // Режим 03h — текстовый 80×25
    Mode13h,   // Режим 13h — графический 320×200, 256 цветов
}

#[derive(Debug)]
pub struct TextBuffer {
    pub data: [u16; 80 * 25], // 4000 ячеек (символ + атрибут)
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            data: [0x0720; 80 * 25], 
        }
    }
}
#[derive(Debug)]
pub struct VideoSystem {
    pub mode: VideoMode,
    pub framebuffer: Option<FrameBuffer>, // None = текстовый режим
    pub text_buffer: TextBuffer,          // <-- Добавляем текстовый буфер
    pub dirty: bool,                      // Флаг обновления для рендеринга
}

impl VideoSystem {
    pub fn new() -> Self {
        Self {
            mode: VideoMode::Text80x25,
            framebuffer: None,
            text_buffer: TextBuffer::new(),
            dirty: false,
        }
    }

    pub fn set_mode(&mut self, mode: VideoMode) {
        self.mode = mode;
        self.framebuffer = match mode {
            VideoMode::Mode13h => Some(FrameBuffer::new()),
            VideoMode::Text80x25 => None,
        };
        if mode == VideoMode::Text80x25 {
            self.text_buffer = TextBuffer::new(); // Сбрасываем буфер при смене режима
        }
        self.dirty = true;
    }

    pub fn write_pixel(&mut self, x: u16, y: u16, color: u8) {
        if let Some(fb) = self.framebuffer.as_mut() {
            if x < 320 && y < 200 {
                let offset = (y * 320 + x) as usize;
                fb.data[offset] = color;
                self.dirty = true;
            }
        }
    }
    
    pub fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty;
    }
}

#[derive(Debug)]
pub struct FrameBuffer {
    pub data: [u8; 320 * 200],
}

impl FrameBuffer {
    pub fn new() -> Self {
        Self {
            data: [0u8; 320 * 200],
        }
    }
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

/// Загрузка стандартной VGA палитры (256 цветов)
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
                    let color = if (pixel_row >> (7 - x)) & 1 == 1 { fg_color } else { bg_color };
                    
                    let px = col * 8 + x;
                    let py = row * 16 + y;
                    pixels[py * WIDTH + px] = color;
                }
            }
        }
    }
    pixels
}

mod fonts_vga8x16;
pub fn get_fonts_vga8x16() -> [u8; 4096] {
    VGA_FONT_8X16
}

pub fn scale_buffer(src: &[u32], src_width: usize, src_height: usize, dst_width: usize, dst_height: usize) -> Vec<u32> {
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