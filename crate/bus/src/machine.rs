// Ver: 4 File: crate/bus/src/machine.rs

/// Трейт Machine (Системная шина / Материнская плата).
/// CPU не должен знать о том, как устроена память или периферия.
/// Он просто запрашивает чтение/запись по адресу или порту.
pub trait Machine {
    // === Работа с физической памятью ===
    fn read_mem_u8(&self, addr: u32) -> u8;
    fn write_mem_u8(&mut self, addr: u32, val: u8);

    fn read_mem_u16(&self, addr: u32) -> u16 {
        let lo = self.read_mem_u8(addr) as u16;
        let hi = self.read_mem_u8(addr.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    fn write_mem_u16(&mut self, addr: u32, val: u16) {
        self.write_mem_u8(addr, val as u8);
        self.write_mem_u8(addr.wrapping_add(1), (val >> 8) as u8);
    }

    fn read_mem_u32(&self, addr: u32) -> u32 {
        let lo = self.read_mem_u16(addr) as u32;
        let hi = self.read_mem_u16(addr.wrapping_add(2)) as u32;
        lo | (hi << 16)
    }

    fn write_mem_u32(&mut self, addr: u32, val: u32) {
        self.write_mem_u16(addr, val as u16);
        self.write_mem_u16(addr.wrapping_add(2), (val >> 16) as u16);
    }

    // === Порты ввода-вывода ===
    fn in_port(&mut self, port: u16) -> u8;
    fn out_port(&mut self, port: u16, val: u8);

    // === Прерывания ===
    /// Аппаратное прерывание от устройства (таймер, клавиатура).
    fn trigger_irq(&mut self, irq_line: u8);

    /// Программное прерывание (инструкция INT).
    /// Возвращает `true`, если эмулятор обработал прерывание
    /// внутренне (например, INT 21h → DOS API).
    /// Возвращает `false`, если нужно прыгать по адресу из IVT.
    fn handle_software_interrupt(&mut self, _vector: u8) -> bool {
        false
    }

    // === Состояние системы ===
    fn is_halted(&self) -> bool;
    fn halt(&mut self);

    fn in_port16(&mut self, port: u16) -> u16 {
        let lo = self.in_port(port) as u16;
        let hi = self.in_port(port.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    fn in_port32(&mut self, port: u16) -> u32 {
        let lo = self.in_port16(port) as u32;
        let hi = self.in_port16(port.wrapping_add(2)) as u32;
        lo | (hi << 16)
    }

    fn out_port16(&mut self, port: u16, val: u16) {
        self.out_port(port, val as u8);
        self.out_port(port.wrapping_add(1), (val >> 8) as u8); // Пишем в port+1
    }

    fn out_port32(&mut self, port: u16, val: u32) {
        self.out_port16(port, val as u16);
        self.out_port16(port.wrapping_add(2), (val >> 16) as u16);
    }
}
