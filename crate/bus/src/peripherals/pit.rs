// Ver: 1 File: crate/bus/src/peripherals/pit.rs
use crate::Peripheral;

/// Эмуляция программируемого интервального таймера 8253/8254.
/// Три канала: 0 — системный таймер (IRQ0), 1 — refresh (заглушка),
/// 2 — PC speaker (заглушка). Порты данных 0x40–0x42, управление 0x43.
#[derive(Debug)]
pub struct Pit {
    pub clock_ticks: u64,
    pub channel_0: u16,
    pub channel_1: u16,
    pub channel_2: u16,
    pub control: u8,
    pub initialized: bool,
}

impl Pit {
    pub fn new() -> Self {
        Self {
            clock_ticks: 0,
            channel_0: 0,
            channel_1: 0,
            channel_2: 0,
            control: 0,
            initialized: false,
        }
    }

    pub fn channel(&self, channel: u8) -> u16 {
        match channel {
            0 => self.channel_0,
            1 => self.channel_1,
            2 => self.channel_2,
            _ => 0,
        }
    }

    pub fn set_channel(&mut self, channel: u8, value: u8) {
        match channel {
            0 => self.channel_0 = (self.channel_0 & 0xFF00) | value as u16,
            1 => self.channel_1 = (self.channel_1 & 0xFF00) | value as u16,
            2 => self.channel_2 = (self.channel_2 & 0xFF00) | value as u16,
            _ => {}
        }
    }

    /// Сообщает IRQ0 при переполнении счётчика канала 0 (упрощённо — каждый тик).
    pub fn tick(&mut self) -> bool {
        self.clock_ticks = self.clock_ticks.wrapping_add(1);
        true
    }
}

impl Default for Pit {
    fn default() -> Self {
        Self::new()
    }
}

impl Peripheral for Pit {
    fn port_read(&mut self, port: u16) -> u8 {
        match port {
            0x40..=0x42 => {
                // Возвращаем текущее «время» в мс, как делал старый dos_core
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                (now.as_millis() & 0xFF) as u8
            }
            _ => 0,
        }
    }

    fn port_write(&mut self, port: u16, val: u8) {
        match port {
            0x40..=0x42 => {
                let channel = (port - 0x40) as u8;
                self.set_channel(channel, val);
                log::info!("PIT: channel {} data = {:#04x}", channel, val);
            }
            0x43 => {
                self.control = val;
                let sc = (val >> 6) & 0x03;
                let mode = (val >> 1) & 0x07;
                let rw = (val >> 4) & 0x03;
                if sc == 0 && mode == 2 && rw == 3 {
                    self.initialized = true;
                    log::info!("PIT: channel 0 initialized for periodic IRQ0");
                }
            }
            _ => {}
        }
    }

    fn tick(&mut self) {
        self.clock_ticks = self.clock_ticks.wrapping_add(1);
    }
}
