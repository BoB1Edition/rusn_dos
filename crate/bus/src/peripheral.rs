// Ver: 1 File: crate/bus/src/peripheral.rs

pub trait Peripheral {
    fn port_read(&mut self, port: u16) -> u8 { 0 }
    fn port_write(&mut self, port: u16, val: u8) {}

    fn mem_read(&mut self, addr: u32) -> u8 { 0 }
    fn mem_write(&mut self, addr: u32, val: u8) {}
    
    /// Тик синхронизации (например, для таймеров).
    fn tick(&mut self) {}
}