// Ver: 1 File: crate/bus/src/peripheral.rs

/// Трейт для периферийных устройств (Keyboard, Video, PIC, Timer).
/// Позволяет "Материнской плате" опрашивать устройства и передавать им данные.
pub trait Peripheral {
    /// Чтение из порта устройства (если оно поддерживает порты).
    fn port_read(&mut self, port: u16) -> u8 { 0 }
    
    /// Запись в порт устройства.
    fn port_write(&mut self, port: u16, val: u8) {}
    
    /// Тик синхронизации (например, для таймеров).
    fn tick(&mut self) {}
}