// Ver: 2 File: crate/dos/src/interrupts/bios.rs
use std::io::Write;

use bus::Machine;
use bus::peripherals::video::VideoMode;
use x86::flags;

use crate::consts;
use crate::system::DosSystem;

fn mode_number(mode: VideoMode) -> u8 {
    match mode {
        VideoMode::Text80x25 => 0x03,
        VideoMode::Mode13h => 0x13,
    }
}

pub fn handle_int10(sys: &mut DosSystem) {
    match sys.cpu.registers.ah() {
        0x00 => {
            // AH=00h: установка видеорежима
            let mode = sys.cpu.registers.al();
            if sys.machine.video.set_mode_u8(mode) {
                log::info!("Video mode set to {:02X}h", mode);
            }
        }
        0x0E => {
            // AH=0Eh: телетайпный вывод символа
            let ch = sys.cpu.registers.al();

            if sys.machine.video.mode == VideoMode::Text80x25 {
                // В текстовом режиме BIOS использует текущий атрибут символа под курсором
                let idx = sys.machine.video.cursor_row as usize * 80
                    + sys.machine.video.cursor_col as usize;
                let attr = (sys.machine.video.text_buffer[idx] >> 8) as u8;

                // Вызываем нашу новую логику телетайпа
                sys.machine.video.write_tty(ch, attr);

                // Дублируем вывод в консоль хоста (для удобства отладки)
                match ch {
                    0x0D => print!("\r"),
                    0x0A => println!(),
                    0x08 => print!("\x08"),
                    c if c >= 0x20 => print!("{}", c as char),
                    _ => {}
                }
                std::io::stdout().flush().ok();
            } else {
                log::debug!("INT 10h/0E in Mode13h: char {:#02x}", ch);
            }
        }
        0x02 => {
            // AH=02h: установка позиции курсора — заглушка
        }
        0x0F => {
            sys.cpu
                .registers
                .set_al(mode_number(sys.machine.video.mode));
            sys.cpu.registers.set_ah(80);
            sys.cpu.registers.set_bh(0);
        }
        _ => {
            log::info!("Unsupported INT 10h / AH={:02X}", sys.cpu.registers.ah());
        }
    }
}

pub fn handle_int16(sys: &mut DosSystem) {
    match sys.cpu.registers.ah() {
        0x00 | 0x10 => {
            if let Some(key) = sys.machine.keyboard.pop_key() {
                sys.cpu.registers.set_ax(key);
            } else {
                sys.cpu.registers.set_ax(0x0000);
            }
        }
        0x01 => {
            if let Some(key) = sys.machine.keyboard.peek_key() {
                sys.cpu.registers.set_ax(key);
                let mut f = sys.cpu.registers.flags();
                f &= !flags::ZF; // ZF = 0
                sys.cpu.registers.set_flags(f);
            } else {
                let mut f = sys.cpu.registers.flags();
                f |= flags::ZF; // ZF = 1
                sys.cpu.registers.set_flags(f);
            }
        }
        0x02 => {
            sys.cpu.registers.set_al(sys.machine.keyboard.shift_flags);
        }
        0x12 => {
            sys.cpu.registers.set_al(sys.machine.keyboard.shift_flags);
        }
        _ => {
            log::warn!("Unsupported INT 16h / AH={:02X}", sys.cpu.registers.ah());
        }
    }
}

pub fn handle_int15(sys: &mut DosSystem) {
    let ax = sys.cpu.registers.ax();

    match ax {
        0x8800..=0x88FF => {
            let mut f = sys.cpu.registers.flags();
            f &= !flags::CF; // CF=0 (Success)
            sys.cpu.registers.set_flags(f);
            let total_kb = consts::DOS_MEMORY_SIZE / 1024;
            let ext_mem_kb = total_kb.saturating_sub(1024);
            sys.cpu.registers.set_ax(ext_mem_kb as u16);
            log::info!("INT 15h/AH=88h: Extended Memory = {:#04X} KB", ext_mem_kb);
        }
        0x2400 => {
            let status = if sys.machine.a20_enabled { 1 } else { 0 };
            let mut f = sys.cpu.registers.flags();
            f &= !flags::CF;
            sys.cpu.registers.set_flags(f);
            sys.cpu.registers.set_al(status);
            log::info!("INT 15h/AX=2400h: A20 status query -> {}", status);
        }
        0x2401 => {
            sys.machine.a20_enabled = true;
            let mut f = sys.cpu.registers.flags();
            f &= !flags::CF;
            sys.cpu.registers.set_flags(f);
            sys.cpu.registers.set_ah(0);
            log::info!("INT 15h/AX=2401h: A20 gate ENABLED via BIOS");
        }
        0x2402 => {
            sys.machine.a20_enabled = false;
            let mut f = sys.cpu.registers.flags();
            f &= !flags::CF;
            sys.cpu.registers.set_flags(f);
            sys.cpu.registers.set_ah(0);
            log::info!("INT 15h/AX=2402h: A20 gate DISABLED via BIOS");
        }
        0x2403 => {
            let status = if sys.machine.a20_enabled { 1 } else { 0 };
            let mut f = sys.cpu.registers.flags();
            f &= !flags::CF;
            sys.cpu.registers.set_flags(f);
            sys.cpu.registers.set_ah(status);
            log::info!("INT 15h/AX=2403h: A20 status = {}", status);
        }
        _ => {
            let mut f = sys.cpu.registers.flags();
            f |= flags::CF;
            sys.cpu.registers.set_flags(f);
            log::warn!("INT 15h/AX={:#04x}: unsupported", ax);
            sys.cpu.registers.set_ah(0x86);
        }
    }
}

pub fn handle_int1a(sys: &mut DosSystem) {
    match sys.cpu.registers.ah() {
        0x00 => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            let ticks = (now.as_millis() * 18 / 1000) as u32; // ~18.2 Гц
            sys.cpu.registers.set_cx((ticks >> 16) as u16);
            sys.cpu.registers.set_dx(ticks as u16);
            sys.cpu.registers.set_al(0);
            let mut f = sys.cpu.registers.flags();
            f &= !flags::CF;
            sys.cpu.registers.set_flags(f);
            log::info!("INT 1Ah / AH=00h: Read system clock -> {:08X} ticks", ticks);
        }
        0x01 => {
            let mut f = sys.cpu.registers.flags();
            f &= !flags::CF;
            sys.cpu.registers.set_flags(f);
            log::info!("INT 1Ah / AH=01h: Set system clock (ignored)");
        }
        _ => {
            log::warn!("Unsupported INT 1Ah / AH={:02X}", sys.cpu.registers.ah());
            let mut f = sys.cpu.registers.flags();
            f |= flags::CF;
            sys.cpu.registers.set_flags(f);
        }
    }
}

pub fn handle_int08(sys: &mut DosSystem) {
    let vector_1c = 0x1C;
    let ivt_addr = (vector_1c as u32) * 4;
    let handler_ip = sys.machine.read_mem_u16(ivt_addr);
    let handler_cs = sys.machine.read_mem_u16(ivt_addr + 2);
    if handler_cs != 0xF000 && (handler_ip != 0 || handler_cs != 0) {
        sys.raise_interrupt(vector_1c);
    }
    sys.machine.out_port(0x20, 0x20);
    log::info!("INT 8h (IRQ0) handled");
}

pub fn handle_int12(sys: &mut DosSystem) {
    // Возвращаем размер непрерывной low memory в КБ (стандартно 640 КБ)
    let memory_kb = 640u16;
    sys.cpu.registers.set_ax(memory_kb);
    let mut f = sys.cpu.registers.flags();
    f &= !flags::CF;
    sys.cpu.registers.set_flags(f);
    log::info!("INT 12h: Returned low memory size = {} KB", memory_kb);
}
