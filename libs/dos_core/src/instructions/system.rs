// Ver: 4
use log::{error, warn};

use crate::{interrupts::bios, machine::DosMachine};

pub(crate) fn int(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let vector = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    bytes.push(vector);
    let _ = machine.log_instruction(csip, &bytes);
    machine.registers.step(None);
    match vector {
        0x21 => machine.handle_int21(),
        0x2F => machine.handle_int2f(),
        0x20 => machine.halted = true,
        0x10 => bios::handle_int10(machine),
        0x16 => bios::handle_int16(machine),
        _ => {
            error!("Unsupported interrupt: INT {:#02X}", vector);
            machine.halted = true;
        }
    }
}

pub fn in_al_dx(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    bytes.push(0xEC); // опкод IN AL, DX
    
    let port = machine.registers.dx();
    let value = match port {
        0x60 => {
            // Порт клавиатуры (8042) — возвращаем сканкод клавиши
            // Для демо-режима возвращаем '1' (сканкод 0x02) с периодической сменой
            let tick = machine.registers.ip() as u64 / 1000;
            match tick % 4 {
                0 => 0x02, // '1'
                1 => 0x03, // '2'
                2 => 0x04, // '3'
                _ => 0x01, // ESC (для выхода)
            }
        }
        0x40..=0x42 => {
            // Порты таймера 8253/8254 — возвращаем текущее время в мс
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            (now.as_millis() & 0xFF) as u8
        }
        0x20 | 0xA0 => {
            // PIC (Programmable Interrupt Controller) — всегда готов к обслуживанию
            0x00
        }
        0x3C2 | 0x3C4 | 0x3C5 | 0x3CE | 0x3CF => {
            // VGA контроллер — заглушка для совместимости
            0x00
        }
        _ => {
            warn!("IN AL, DX from unimplemented port {:#04x}", port);
            0x00 // заглушка для неизвестных портов
        }
    };
    
    machine.registers.set_al(value);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn nop(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let bytes = prev.to_vec();
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/system.rs
pub fn in_al_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let port = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(port);
    
    let value = match port {
        0x60 => {
            // Порт клавиатуры (8042)
            let tick = machine.registers.ip() as u64 / 1000;
            match tick % 4 {
                0 => 0x02, // '1'
                1 => 0x03, // '2'
                2 => 0x04, // '3'
                _ => 0x01, // ESC
            }
        }
        0x40..=0x42 => {
            // Порты таймера 8253/8254
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            (now.as_millis() & 0xFF) as u8
        }
        0x20 | 0xA0 => {
            // PIC (Programmable Interrupt Controller)
            0x00
        }
        _ => {
            log::warn!("IN AL, imm8 from unimplemented port {:#02x}", port);
            0x00
        }
    };
    
    machine.registers.set_al(value);
    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/system.rs
pub fn hlt(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let bytes = prev.to_vec();
    
    // Останавливаем выполнение программы
    // В реальном железе процессор ждёт прерывания, но в эмуляторе мы завершаем выполнение
    log::info!("HLT instruction executed at CS:IP={:#04x}:{:#04x}, halting CPU", 
               machine.registers.cs(), machine.registers.ip());
    machine.halted = true;
    
    machine.log_instruction(csip, &bytes).ok();
}