// Ver: 1 File: crate/bus/src/peripherals/pic.rs
use crate::peripheral::Peripheral;

/// Эмуляция пары каскадных контроллеров прерываний 8259A.
/// Мастер: порты 0x20/0x21 (IRQ0–IRQ7). Слейв: порты 0xA0/0xA1 (IRQ8–IRQ15).
#[derive(Debug)]
pub struct Pic {
    pub master_mask: u8,
    pub slave_mask: u8,
    pub master_irr: u8, // Interrupt Request Register
    pub master_isr: u8, // In-Service Register
    pub slave_irr: u8,
    pub slave_isr: u8,
}

impl Pic {
    pub fn new() -> Self {
        Self {
            master_mask: 0xFF, // По умолчанию все замаскированы
            slave_mask: 0xFF,
            master_irr: 0,
            master_isr: 0,
            slave_irr: 0,
            slave_isr: 0,
        }
    }

    pub fn trigger_irq(&mut self, irq: u8) {
        match irq {
            0..=7 => self.master_irr |= 1 << irq,
            8..=15 => {
                self.slave_irr |= 1 << (irq - 8);
                self.master_irr |= 1 << 2; // слейв подключён к IRQ2
            }
            _ => {}
        }
    }

    pub fn get_next_interrupt(&mut self) -> Option<u8> {
        let pending = self.master_irr & !self.master_mask;
        if pending == 0 {
            return None;
        }
        for i in 0..8 {
            if (pending & (1 << i)) != 0 {
                self.master_irr &= !(1 << i);
                self.master_isr |= 1 << i;
                return Some(i);
            }
        }
        None
    }
}

impl Default for Pic {
    fn default() -> Self {
        Self::new()
    }
}

impl Peripheral for Pic {
    fn port_read(&mut self, port: u16) -> u8 {
        match port {
            0x20 => self.master_isr, // Упрощенно
            0x21 => self.master_mask,
            0xA0 => self.slave_isr,
            0xA1 => self.slave_mask,
            _ => 0,
        }
    }

    fn port_write(&mut self, port: u16, val: u8) {
        match port {
            0x20 => {
                if val == 0x20 {
                    // End of Interrupt (EOI) - сбрасываем старший бит ISR
                    let highest_isr = self.master_isr & (!(self.master_isr.wrapping_sub(1)));
                    self.master_isr &= !highest_isr;
                }
            }
            0x21 => self.master_mask = val,
            0xA0 => {
                if val == 0x20 {
                    let highest_isr = self.slave_isr & (!(self.slave_isr.wrapping_sub(1)));
                    self.slave_isr &= !highest_isr;
                }
            }
            0xA1 => self.slave_mask = val,
            _ => {}
        }
    }
}
