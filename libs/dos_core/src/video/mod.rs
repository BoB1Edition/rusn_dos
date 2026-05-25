// Ver: 1 File: ./libs/dos_core/src/video/mod.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoMode {
    Text80x25, // Режим 03h — текстовый 80×25
    Mode13h,   // Режим 13h — графический 320×200, 256 цветов
}

#[derive(Debug)]
pub struct VideoSystem {
    pub mode: VideoMode,
    pub framebuffer: Option<FrameBuffer>, // None = текстовый режим
    pub dirty: bool,                      // Флаг обновления для рендеринга
}

impl VideoSystem {
    pub fn new() -> Self {
        Self {
            mode: VideoMode::Text80x25,
            framebuffer: None,
            dirty: false,
        }
    }

    pub fn set_mode(&mut self, mode: VideoMode) {
        self.mode = mode;
        self.framebuffer = match mode {
            VideoMode::Mode13h => Some(FrameBuffer::new()),
            VideoMode::Text80x25 => None,
        };
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
