// Ver: 1

use std::{fs::File, io::Write, ops::{Index, IndexMut}};

const DOS_MEMORY_SIZE: usize = 0x110000;

#[derive(Debug, Clone)]
pub struct Memory {
    data: Box<[u8]>,
    size: usize,
}

impl Memory {
    /*pub(crate) fn print_data(&self, stage: u8) {
        log::debug!("{:?}", self.data);
        let mut file = File::create(format!("memory_stage{}", stage)).ok().unwrap();
        file.write_all(&self.data);
        file.flush();
    }*/
    pub fn new() -> Self {        
        Self {
            data: vec![0u8; DOS_MEMORY_SIZE].into_boxed_slice(),
            size: DOS_MEMORY_SIZE,
        }
    }
    pub fn from_slice(slice: Box<[u8]>) -> Self {
        let size = slice.len();
        Self {
            data: slice,
            size: size,
        }
    }

    #[inline(always)]
    pub fn read_u8(&self, addr: u32) -> u8 {
        if addr >= self.size as u32 {
            log::warn!("Memory read out of bounds: {:#x}", addr);
            return 0;
        }

        if addr == 0x000000 || addr == 0x100000 || addr >= 0x100000 {
            log::debug!(
                "MEM READ: addr={:#x}, value={:#02x}",
                addr,
                self.data[addr as usize]
            );
        }

        self.data[addr as usize]
    }

    #[inline(always)]
    pub fn write_u8(&mut self, addr: u32, value: u8) {
        if addr >= self.size as u32 {
            log::warn!("Memory write out of bounds: {:#x}", addr);
            return;
        }

        if addr == 0x000000 || addr == 0x100000 || addr >= 0x100000 {
            log::debug!("MEM WRITE: addr={:#x}, value={:#02x}", addr, value);
        }

        self.data[addr as usize] = value;
    }

    #[inline(always)]
    pub fn read_u16(&self, addr: u32) -> u16 {
        let lo = self.read_u8(addr) as u16;
        let hi = self.read_u8(addr.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    #[inline(always)]
    pub fn write_u16(&mut self, addr: u32, value: u16) {
        self.write_u8(addr, value as u8);
        self.write_u8(addr.wrapping_add(1), (value >> 8) as u8);
    }

    #[inline(always)]
    pub fn read_u32(&self, addr: u32) -> u32 {
        let lo = self.read_u16(addr) as u32;
        let hi = self.read_u16(addr.wrapping_add(2)) as u32;
        lo | (hi << 16)
    }

    #[inline(always)]
    pub fn write_u32(&mut self, addr: u32, value: u32) {
        self.write_u16(addr, value as u16);
        self.write_u16(addr.wrapping_add(2), (value >> 16) as u16);
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.size]
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data[..self.size]
    }

    pub fn len(&self) -> usize {
        self.size
    }
}

impl Index<usize> for Memory {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.size {
            panic!(
                "Memory access out of bounds: 0x{:05X} >= 0x{:05X}",
                index, self.size
            );
        }
        &self.data[index]
    }
}

impl IndexMut<usize> for Memory {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.size {
            panic!(
                "Memory access out of bounds: 0x{:05X} >= 0x{:05X}",
                index, self.size
            );
        }
        &mut self.data[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_around() {
        let mut mem = Memory::new();
        mem.write_u8(0xFFFFF, 0xAA);
        assert_eq!(mem.read_u8(0x100000), 0xAA);
        assert_eq!(mem.read_u8(0x100001), mem.read_u8(0x00001));
    }

    #[test]
    fn test_word_across_boundary() {
        let mut mem = Memory::new();
        mem.write_u8(0xFFFFF, 0x12);
        mem.write_u8(0x00000, 0x34);
        assert_eq!(mem.read_u16(0xFFFFF), 0x3412);
    }
}
