// Ver: 3 File: ./libs/dos_core/src/interrupts/bios.rs
use std::io::Write;

use crate::video::VideoMode;
use crate::{DosMachine, consts, flags};

pub(crate) fn handle_int10(machine: &mut DosMachine) {
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
                        log::debug!("INT 10h/0E in Mode13h: char {:#02x}", ch);
                    }
                }
            }
        }
        0x0F => {
            machine.registers.set_al(machine.video.mode as u8);
            machine.registers.set_ah(80); // ширина экрана
            machine.registers.set_bh(0); // активная страница
        }
        _ => {
            log::info!("Unsupported INT 10h / AH={:02X}", machine.registers.ah());
        }
    }
}

pub(crate) fn handle_int16(machine: &mut DosMachine) {
    match machine.registers.ah() {
        0x00 | 0x10 => {
            if let Some(key) = machine.keyboard.pop_key() {
                machine.registers.set_ax(key);
            } else {
                machine.registers.set_ax(0x0000);
            }
        }
        0x01 => {
            if let Some(key) = machine.keyboard.peek_key() {
                machine.registers.set_ax(key);
                let mut flags = machine.registers.flags();
                flags &= !crate::flags::ZF; // ZF = 0
                machine.registers.set_flags(flags);
            } else {
                let mut flags = machine.registers.flags();
                flags |= crate::flags::ZF; // ZF = 1
                machine.registers.set_flags(flags);
            }
        }
        0x02 => {
            machine.registers.set_al(machine.keyboard.shift_flags);
        }
        0x12 => {
            machine.registers.set_al(machine.keyboard.shift_flags);
        }
        _ => {
            log::warn!("Unsupported INT 16h / AH={:02X}", machine.registers.ah());
        }
    }
}

pub(crate) fn handle_int15(machine: &mut DosMachine) {
    let ax = machine.registers.ax();

    match ax {
        0x8800..=0x88FF => {
            let mut flags = machine.registers.flags();
            flags &= !flags::CF; // CF=0 (Success)
            machine.registers.set_flags(flags);
            let total_kb = consts::DOS_MEMORY_SIZE / 1024;
            let ext_mem_kb = total_kb.saturating_sub(1024);
            machine.registers.set_ax(ext_mem_kb as u16);
            log::info!("INT 15h/AH=88h: Extended Memory = {:#04X} KB", ext_mem_kb);
        }
        0x2400 => {
            let status = if machine.a20_enabled { 1 } else { 0 };
            let mut flags = machine.registers.flags();
            flags &= !(flags::CF); // CF = 0
            machine.registers.set_flags(flags);
            machine.registers.set_al(status);
            log::info!("INT 15h/AX=2400h: A20 status query -> {}", status);
        }
        0x2401 => {
            machine.a20_enabled = true;
            let mut flags = machine.registers.flags();
            flags &= !(flags::CF);
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
            machine.registers.set_ah(1);
            log::info!("INT 15h/AX=2403h: A20 status = {}", status);
        }
        _ => {
            let mut flags = machine.registers.flags();
            flags |= flags::CF;
            machine.registers.set_flags(flags);
            log::warn!("INT 15h/AX={:#04x}: unsupported", ax);
            machine.registers.set_ah(0x86);
            //machine.halted=true;
        }
    }
}

pub(crate) fn handle_int1a(machine: &mut DosMachine) {
    match machine.registers.ah() {
        0x00 => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            let ticks = (now.as_millis() * 18 / 1000) as u32; // ~18.2 Гц
            machine.registers.set_cx((ticks >> 16) as u16);
            machine.registers.set_dx(ticks as u16);
            machine.registers.set_al(0); // Флаг перехода через полночь = 0
            let mut flags = machine.registers.flags();
            flags &= !(flags::CF); // CF = 0
            machine.registers.set_flags(flags);
            log::info!("INT 1Ah / AH=00h: Read system clock -> {:08X} ticks", ticks);
        }
        0x01 => {
            let mut flags = machine.registers.flags();
            flags &= !(flags::CF);
            machine.registers.set_flags(flags);
            log::info!("INT 1Ah / AH=01h: Set system clock (ignored)");
        }
        _ => {
            log::warn!("Unsupported INT 1Ah / AH={:02X}", machine.registers.ah());
            let mut flags = machine.registers.flags();
            flags |= flags::CF; // CF=1 (ошибка)
            machine.registers.set_flags(flags);
        }
    }
}

pub(crate) fn handle_int08(machine: &mut DosMachine) {
    let vector_1c = 0x1C;
    let ivt_addr = (vector_1c as u32) * 4;
    let handler_ip = machine.read_phys_u16(ivt_addr);
    let handler_cs = machine.read_phys_u16(ivt_addr + 2);
    if handler_cs != 0xF000 && (handler_ip != 0 || handler_cs != 0) {
        crate::instructions::system::call_interrupt(machine, vector_1c);
    }
    machine.out_imm8_al(0x20, 0x20);
    log::info!("INT 8h (IRQ0) handled");
}

pub(crate) fn handle_int12(machine: &mut DosMachine) {
    // Возвращаем размер непрерывной low memory в КБ (стандартно 640 КБ)
    let memory_kb = 640u16;
    machine.registers.set_ax(memory_kb);
    let mut flags = machine.registers.flags();
    flags &= !(flags::CF); // CF=0 (успех)
    machine.registers.set_flags(flags);
    log::info!("INT 12h: Returned low memory size = {} KB", memory_kb);
}
