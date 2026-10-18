// Ver: 1 File: crate/bus/src/peripheral.rs

/// Трейт для периферийных устройств (Keyboard, Video, PIC, Timer).
/// Позволяет "Материнской плате" опрашивать устройства и передавать им данные.
pub trait Peripheral {
    // === I/O Ports (x86-specific) ===

    /// Чтение из порта устройства.
    fn port_read(&mut self, _port: u16) -> u8 {
        0
    }

    /// Запись в порт устройства.
    fn port_write(&mut self, _port: u16, _val: u8) {}

    // === Memory-Mapped I/O (универсально для всех систем) ===

    /// Чтение из адресного пространства устройства.
    /// `addr` — относительный адрес внутри окна устройства.
    fn mem_read(&mut self, _addr: u32) -> u8 {
        0
    }

    /// Запись в адресное пространство устройства.
    fn mem_write(&mut self, _addr: u32, _val: u8) {}

    // === Синхронизация ===

    /// Тик синхронизации (таймеры, генераторы кадров).
    fn tick(&mut self) {}
}