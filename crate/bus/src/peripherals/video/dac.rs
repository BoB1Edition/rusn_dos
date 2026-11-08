use super::types::VideoMode;

pub fn ac_defaults(mode: VideoMode) -> [u8; 0x15] {
    let mut r = [0u8; 0x15];
    for i in 0..16 {
        r[i] = i as u8;
    }
    r[0x10] = match mode {
        VideoMode::Text80x25 => 0x0C,
        VideoMode::Mode13h => 0x41,
        _ => 0x01,
    };
    r[0x11] = 0x00;
    r[0x12] = 0x0F;
    r[0x13] = 0x00;
    r[0x14] = 0x00;
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
pub fn six_to_eight(v: u8) -> u8 {
    (v << 2) | (v >> 4)
}
