// Ver: 2
use std::io::Write;

use crate::{DosMachine};
use crate::video::VideoMode;

pub fn handle_int10(machine: &mut DosMachine) {
    match machine.registers.ah() {
        0x00 => {
            // AH=00h: установка видеорежима
            let mode = match machine.registers.al() {
                0x03 => VideoMode::Text80x25,
                0x13 => VideoMode::Mode13h,
                _ => {
                    log::warn!("Unsupported video mode {:02X}h", machine.registers.al());
                    return;
                }
            };
            machine.video.set_mode(mode);
            log::info!("Video mode set to {:?}", mode);
        }
        0x0E => {
            // AH=0Eh: телетайпный вывод символа (только для текстового режима)
            if machine.video.mode == VideoMode::Text80x25 {
                let ch = machine.registers.al() as char;
                print!("{}", ch);
                std::io::stdout().flush().ok();
            }
            // В режиме 13h игнорируем (реальные игры используют прямую запись в видеопамять)
        }
        0x0F => {
            // AH=0Fh: запрос текущего состояния
            machine.registers.set_al(machine.video.mode as u8);
            machine.registers.set_ah(80); // ширина экрана
            machine.registers.set_bh(0);  // активная страница
        }
        _ => {
            log::debug!("Unsupported INT 10h / AH={:02X}", machine.registers.ah());
        }
    }
}

// libs/dos_core/src/interrupts/bios.rs
pub fn handle_int16(machine: &mut DosMachine) {
    match machine.registers.ah() {
        0x00 | 0x10 => {
            machine.registers.set_ax(0x3100); // '1' + сканкод
        }
        _ => {
            log::debug!("Unsupported INT 16h / AH={:02X}", machine.registers.ah());
        }
    }
}