// Ver: 1
use std::io::Write;

use crate::video::VideoMode;
use crate::{DosMachine, flags};

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
            // AH=0Eh: телетайпный вывод символа
            let ch = machine.registers.al();
            match ch {
                0x08 => { // Backspace
                    // В простейшем эмуляторе можно игнорировать или сдвигать курсор
                }
                0x0D => { // Carriage Return (игнорируем в графике, или сбрасываем X)
                }
                0x0A => { // Line Feed (сдвиг вниз по пиксельной сетке сложно, пропустим)
                }
                _ => {
                    // В текстовом режиме пишем в консоль
                    if machine.video.mode == VideoMode::Text80x25 {
                        print!("{}", ch as char);
                        std::io::stdout().flush().ok();
                    } else {
                        // В Mode13h пишем в видеопамять 0xB8000 (текстовый буфер VGA)
                        // Для простоты: выводим символ в лог/консоль, но помечаем dirty,
                        // если хотим эмулировать текстовый оверлей.
                        // Реальные игры в 13h обычно рисуют сами, но для совместимости:
                        log::debug!("INT 10h/0E in Mode13h: char {:#02x}", ch);
                    }
                }
            }
        }
        0x0F => {
            // AH=0Fh: запрос текущего состояния
            machine.registers.set_al(machine.video.mode as u8);
            machine.registers.set_ah(80); // ширина экрана
            machine.registers.set_bh(0); // активная страница
        }
        _ => {
            log::info!("Unsupported INT 10h / AH={:02X}", machine.registers.ah());
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
            log::info!("Unsupported INT 16h / AH={:02X}", machine.registers.ah());
        }
    }
}

pub fn handle_int15(machine: &mut DosMachine) {
    let ax = machine.registers.ax();

    match ax {
        0x2401 => {
            // Enable A20 gate
            machine.a20_enabled = true;
            // CF=0 (успех), AH=0
            let mut flags = machine.registers.flags();
            flags &= !(flags::CF); // CF = 0
            machine.registers.set_flags(flags);
            machine.registers.set_ah(0);
            log::info!("INT 15h/AX=2401h: A20 gate ENABLED via BIOS");
        }
        0x2402 => {
            // Disable A20 gate
            machine.a20_enabled = false;
            let mut flags = machine.registers.flags();
            flags &= !(flags::CF); // CF = 0
            machine.registers.set_flags(flags);
            machine.registers.set_ah(0);
            log::info!("INT 15h/AX=2402h: A20 gate DISABLED via BIOS");
        }
        0x2403 => {
            // Query A20 status
            let status = if machine.a20_enabled { 1 } else { 0 };
            let mut flags = machine.registers.flags();
            flags &= !(flags::CF); // CF = 0
            machine.registers.set_flags(flags);
            machine.registers.set_al(status);
            log::info!("INT 15h/AX=2403h: A20 status = {}", status);
        }
        _ => {
            // Неизвестная функция — возвращаем ошибку
            let mut flags = machine.registers.flags();
            flags |= flags::CF; // CF = 1 (ошибка)
            machine.registers.set_flags(flags);
            machine.registers.set_ah(0x86); // Function not supported
            log::warn!("INT 15h/AX={:#04x}: unsupported", ax);
        }
    }
}
