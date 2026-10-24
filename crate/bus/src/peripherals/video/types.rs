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
        matches!(self, Self::Mode112_640x480x24bit)
    }
}
