// Ver: 2 File: crate/bus/src/peripheral.rs

pub trait Peripheral {
    fn port_read(&mut self, _port: u16) -> u8 {
        0
    }

    fn port_write(&mut self, _port: u16, _val: u8) {}
    fn mem_read(&mut self, _addr: u32) -> u8 {
        0
    }

    fn mem_write(&mut self, _addr: u32, _val: u8) {}
    fn tick(&mut self) {}
}
