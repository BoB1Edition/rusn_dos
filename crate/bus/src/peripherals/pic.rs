// Ver: 1 File: crate/bus/src/peripherals/pic.rs
use crate::peripheral::Peripheral;

#[derive(Debug)]
pub struct Pic {
    pub master_mask: u8,
    pub slave_mask: u8,
    pub master_irr: u8, // Interrupt Request Register
    pub master_isr: u8, // In-Service Register
}

impl Pic {
    pub fn new() -> Self {
        Self {
            master_mask: 0xFF, // По умолчанию все замаскированы
            slave_mask: 0xFF,
            master_irr: 0,
            master_isr: 0,
        }
    }

    pub fn trigger_irq(&mut self, irq: u8) {
        if irq < 8 {
            self.master_irr |= 1 << irq;
        }
    }

    pub fn get_next_interrupt(&mut self) -> Option<u8> {
        // Находим непримаскированный и ожидающий IRQ
        let pending = self.master_irr & !self.master_mask;
        if pending == 0 { return None; }
        
        // Простейший приоритет: от IRQ0 до IRQ7
        for i in 0..8 {
            if (pending & (1 << i)) != 0 {
                self.master_irr &= !(1 << i); // Сбрасываем IRR
                self.master_isr |= (1 << i);  // Устанавливаем ISR
                return Some(i);
            }
        }
        None
    }
}

impl Peripheral for Pic {
    fn port_read(&mut self, port: u16) -> u8 {
        match port {
            0x20 => self.master_isr, // Упрощенно
            0x21 => self.master_mask,
            _ => 0,
        }
    }

    fn port_write(&mut self, port: u16, val: u8) {
        match port {
            0x20 => {
                if val == 0x20 {
                    // End of Interrupt (EOI) - сбрасываем старший бит ISR
                    let highest_isr = self.master_isr & (!(self.master_isr - 1));
                    self.master_isr &= !highest_isr;
                }
            }
            0x21 => self.master_mask = val,
            _ => {}
        }
    }
}