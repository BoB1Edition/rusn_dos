// Ver: 1 File: crate/bus/src/memory.rs

/// Базовая оперативная память.
/// В отличие от старой версии, размер не захардкожен в константах,
/// а задается при создании.
#[derive(Debug, Clone)]
pub struct Memory {
    data: Box<[u8]>,
    size: usize,
}

impl Memory {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0u8; size].into_boxed_slice(),
            size,
        }
    }

    pub fn from_slice(data: Box<[u8]>) -> Self {
        let size = data.len();
        Self { data, size }
    }

    #[inline(always)]
    pub fn read_u8(&self, addr: u32) -> u8 {
        if (addr as usize) < self.size {
            self.data[addr as usize]
        } else {
            log::warn!("Memory read out of bounds: {:#x}", addr);
            0
        }
    }

    #[inline(always)]
    pub fn write_u8(&mut self, addr: u32, value: u8) {
        if (addr as usize) < self.size {
            self.data[addr as usize] = value;
        } else {
            log::warn!("Memory write out of bounds: {:#x}", addr);
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
    
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
}