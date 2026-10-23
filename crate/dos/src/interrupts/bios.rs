// Ver: 16 File: crate/dos/src/interrupts/bios.rs
use std::io::Write;

use bus::Machine;
use bus::peripherals::video::{VbeMode, VideoMode};
use x86::flags;

use crate::consts;
use crate::system::DosSystem;

fn mode_number(mode: VideoMode) -> u8 {
    match mode {
        VideoMode::Text80x25 => 0x03,
        VideoMode::Mode13h => 0x13,
        VideoMode::Mode0Dh => 0x0D,
        VideoMode::Mode0Eh => 0x0E,
        VideoMode::Mode10h => 0x10,
        VideoMode::Mode12h => 0x12,
    }
}

fn sync_cursor_bda(sys: &mut DosSystem) {
    sys.machine
        .write_mem_u8(0x450, sys.machine.video.cursor_row);
    sys.machine
        .write_mem_u8(0x451, sys.machine.video.cursor_col);
}

pub fn handle_int10(sys: &mut DosSystem) {
    // Разбор CX/DX на байты без экзотических аксессоров
    let cx = sys.cpu.registers.cx();
    let ch = (cx >> 8) as u8;
    let cl = (cx & 0xFF) as u8;
    let dx = sys.cpu.registers.dx();
    let dh = (dx >> 8) as u8;
    let dl = (dx & 0xFF) as u8;

    match sys.cpu.registers.ah() {
        0x00 => {
            let mode = sys.cpu.registers.al();
            if sys.machine.video.set_mode_u8(mode) {
                log::info!("Video mode set to {:02X}h", mode);
            }
            sync_cursor_bda(sys); // курсор сброшен в 0,0 — отзеркалили
        }

        0x01 => {
            // Форма курсора теперь живёт в CRTC 0x0A/0x0B
            sys.machine.video.crtc_regs[0x0A] = ch & 0x3F;
            sys.machine.video.crtc_regs[0x0B] = cl & 0x1F;
            sys.machine.write_mem_u8(0x460, ch);
            sys.machine.write_mem_u8(0x461, cl);
            sys.machine.video.dirty = true;
        }
        0x02 => {
            let row = dh.min(24);
            let col = dl.min(79);
            sys.machine.video.cursor_row = row;
            sys.machine.video.cursor_col = col;
            // ✅ BIOS пишет и в CRTC, как настоящее железо
            let line = sys.machine.video.text_line_cells();
            sys.machine
                .video
                .set_cursor_cell(row as usize * line + col as usize);
            sync_cursor_bda(sys);
            sys.machine.video.dirty = true;
        }
        0x03 => {
            sys.cpu.registers.set_bh(0);
            sys.cpu.registers.set_dh(sys.machine.video.cursor_row);
            sys.cpu.registers.set_dl(sys.machine.video.cursor_col);
            sys.cpu
                .registers
                .set_ch(sys.machine.video.crtc_regs[0x0A] & 0x3F);
            sys.cpu
                .registers
                .set_cl(sys.machine.video.crtc_regs[0x0B] & 0x1F);
        }

        0x05 => {
            // Выбор страницы: держим только страницу 0
            let page = sys.cpu.registers.al();
            if page != 0 {
                log::warn!(
                    "INT 10h/AH=05h: page {} не поддерживается, остаёмся на 0",
                    page
                );
            }
        }

        0x06 => scroll_window_int(sys, true), // скролл вверх / очистка
        0x07 => scroll_window_int(sys, false), // скролл вниз

        0x08 => {
            // Чтение символа и атрибута под курсором
            let idx =
                sys.machine.video.cursor_row as usize * 80 + sys.machine.video.cursor_col as usize;
            let word = sys.machine.video.text_buffer[idx];
            sys.cpu.registers.set_al((word & 0xFF) as u8);
            sys.cpu.registers.set_ah(((word >> 8) & 0xFF) as u8);
        }

        0x09 => {
            // Запись символа с атрибутом, CX раз, без движения курсора
            let count = cx as usize;
            sys.machine.video.write_cell_repeat(
                sys.cpu.registers.al(),
                Some(sys.cpu.registers.bl()),
                count,
            );
        }

        0x0A => {
            // Запись символа без изменения атрибута, CX раз
            let count = cx as usize;
            sys.machine
                .video
                .write_cell_repeat(sys.cpu.registers.al(), None, count);
        }

        0x0E => {
            // Телетайп: вся логика уже в tty_write (видео + stdout)
            let ch = sys.cpu.registers.al();
            if sys.machine.video.mode == VideoMode::Text80x25 {
                tty_write(sys, ch);
                sync_cursor_bda(sys);
            } else {
                log::debug!("INT 10h/0E in Mode13h: char {:#02x}", ch);
            }
        }
        0x10 => palette_functions(sys),
        0x12 => {
            // Alternate function select: отвечаем «поддержано» на известные BL
            match sys.cpu.registers.bl() {
                0x10 | 0x20 | 0x30 | 0x31 | 0x32 | 0x33 | 0x34 => {
                    sys.cpu.registers.set_al(0x12);
                }
                _ => log::info!(
                    "Unsupported INT 10h / AH=12h BL={:02X}",
                    sys.cpu.registers.bl()
                ),
            }
        }

        0x0F => {
            sys.cpu
                .registers
                .set_al(mode_number(sys.machine.video.mode));
            sys.cpu.registers.set_ah(80);
            sys.cpu.registers.set_bh(0);
        }
        0x4F => {
            // VESA BIOS Extensions
            let sub = sys.cpu.registers.al();
            let es = sys.cpu.registers.es(); // ✅ добавить
            let di = sys.cpu.registers.di(); // ✅ добавить
            let bx = sys.cpu.registers.bx(); // ✅ добавить
            match sub {
                0x00 => {
                    let block = sys.machine.video.vbe_info_block();
                    for (i, &b) in block.iter().enumerate() {
                        sys.write_u8(es, di.wrapping_add(i as u16), b);
                    }
                    sys.cpu.registers.set_ax(0x004F);
                }
                0x01 => match VbeMode::from_number(cx) {
                    Some(m) => {
                        let block = sys.machine.video.vbe_mode_info_block(m);
                        for (i, &b) in block.iter().enumerate() {
                            sys.write_u8(es, di.wrapping_add(i as u16), b);
                        }
                        sys.cpu.registers.set_ax(0x004F);
                    }
                    None => sys.cpu.registers.set_ax(0x014F),
                },
                0x02 => match VbeMode::from_number(bx & 0x1FF) {
                    Some(m) => {
                        sys.machine.video.set_vbe_mode(m);
                        sys.cpu.registers.set_ax(0x004F);
                        log::info!("VBE mode {:#04X} set", bx & 0x1FF);
                    }
                    None => sys.cpu.registers.set_ax(0x014F),
                },
                0x05 => {
                    let bh = sys.cpu.registers.bh();
                    if bh == 0 {
                        sys.machine.video.vbe_bank = (bx as usize).min(11);
                        sys.machine.video.dirty = true;
                        sys.cpu.registers.set_ax(0x004F);
                    } else {
                        sys.cpu.registers.set_dx(sys.machine.video.vbe_bank as u16);
                        sys.cpu.registers.set_ax(0x004F);
                    }
                }
                _ => {
                    log::info!("Unsupported VBE subfunction AL={:#02X}", sub);
                    sys.cpu.registers.set_ax(0x014F);
                }
            }
        }
        _ => {
            log::info!("Unsupported INT 10h / AH={:02X}", sys.cpu.registers.ah());
        }
    }
}

fn scroll_window_int(sys: &mut DosSystem, up: bool) {
    let lines = sys.cpu.registers.al() as usize;
    let attr = sys.cpu.registers.bh();
    let cx = sys.cpu.registers.cx();
    let dx = sys.cpu.registers.dx();
    sys.machine.video.scroll_window(
        (cx >> 8) as usize,   // top
        (cx & 0xFF) as usize, // left
        (dx >> 8) as usize,   // bottom
        (dx & 0xFF) as usize, // right
        lines,
        attr,
        up,
    );
}

pub fn handle_int16(sys: &mut DosSystem) {
    match sys.cpu.registers.ah() {
        0x00 | 0x10 => {
            if let Some(key) = sys.machine.keyboard.pop_key() {
                sys.cpu.registers.set_ax(key);
            } else {
                //sys.cpu.registers.set_ax(0x0000);
                sys.suspend_for_key(0x16);
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
            if sys.machine_config.is_feature_locked("a20_enable") {
                log::warn!("INT 15h/AX=2401h: A20 enable BLOCKED by disable_change_flags");
                let mut f = sys.cpu.registers.flags();
                f |= flags::CF; // Возвращаем ошибку (Carry Flag = 1)
                sys.cpu.registers.set_flags(f);
                sys.cpu.registers.set_ah(0x86); // Функция не поддерживается / ошибка
                return;
            }

            sys.machine.a20_enabled = true;
            let mut f = sys.cpu.registers.flags();
            f &= !flags::CF;
            sys.cpu.registers.set_flags(f);
            sys.cpu.registers.set_ah(0);
            log::info!("INT 15h/AX=2401h: A20 gate ENABLED via BIOS");
        }
        0x2402 => {
            if sys.machine_config.is_feature_locked("a20_enable") {
                log::warn!("INT 15h/AX=2402h: A20 disable BLOCKED by disable_change_flags");
                let mut f = sys.cpu.registers.flags();
                f |= flags::CF;
                sys.cpu.registers.set_flags(f);
                sys.cpu.registers.set_ah(0x86);
                return;
            }

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
    const BDA_TICK_COUNT: u32 = 0x046C;
    const BDA_MIDNIGHT_FLAG: u32 = 0x0470;
    const TICKS_PER_DAY: u32 = 0x0018_00B0;

    let ticks = sys.machine.read_mem_u32(BDA_TICK_COUNT);
    let new_ticks = ticks.wrapping_add(1);

    if new_ticks >= TICKS_PER_DAY {
        sys.machine.write_mem_u32(BDA_TICK_COUNT, 0);
        sys.machine.write_mem_u8(BDA_MIDNIGHT_FLAG, 1);
    } else {
        sys.machine.write_mem_u32(BDA_TICK_COUNT, new_ticks);
    }

    let vector_1c = 0x1C;
    let ivt_addr = (vector_1c as u32) * 4;
    let handler_ip = sys.machine.read_mem_u16(ivt_addr);
    let handler_cs = sys.machine.read_mem_u16(ivt_addr + 2);
    if handler_cs != 0xF000 && (handler_ip != 0 || handler_cs != 0) {
        sys.raise_interrupt(vector_1c);
    }
    sys.machine.out_port(0x20, 0x20);
    log::trace!("INT 8h (IRQ0) handled");
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

pub fn tty_write(sys: &mut DosSystem, ch: u8) {
    if sys.machine.video.mode == VideoMode::Text80x25 {
        let idx =
            sys.machine.video.cursor_row as usize * 80 + sys.machine.video.cursor_col as usize;
        let attr = (sys.machine.video.text_buffer[idx] >> 8) as u8;
        sys.machine.video.write_tty(ch, attr);
        sync_cursor_bda(sys);
    }
    match ch {
        0x0D => print!("\r"),
        0x0A => print!("\n"),
        0x08 => print!("\x08"),
        c if c >= 0x20 => print!("{}", c as char),
        _ => {}
    }
    std::io::stdout().flush().ok();
}

fn palette_functions(sys: &mut DosSystem) {
    let sub = sys.cpu.registers.al();
    let bl = sys.cpu.registers.bl();
    let bh = sys.cpu.registers.bh();
    let bx = sys.cpu.registers.bx();
    let cx = sys.cpu.registers.cx();
    let es = sys.cpu.registers.es();
    let dx = sys.cpu.registers.dx();
    match sub {
        0x00 => {
            // установить палитровый регистр: BL=регистр, BH=значение
            let r = (bl & 0x1F) as usize;
            if r < sys.machine.video.ac_regs.len() {
                sys.machine.video.ac_regs[r] = bh;
                sys.machine.video.dirty = true;
            }
        }
        0x01 => {
            // overscan
            sys.machine.video.ac_regs[0x11] = bh;
            sys.machine.video.dirty = true;
        }
        0x02 => {
            // все 16 регистров + overscan из ES:DX (17 байт)
            for i in 0..16u16 {
                sys.machine.video.ac_regs[i as usize] = sys.read_u8(es, dx.wrapping_add(i));
            }
            sys.machine.video.ac_regs[0x11] = sys.read_u8(es, dx.wrapping_add(16));
            sys.machine.video.dirty = true;
        }
        0x03 => {
            // blink/intensity: BL=0 -> blink, BL=1 -> intensity
            if bl == 0 {
                sys.machine.video.ac_regs[0x10] |= 0x08;
            } else {
                sys.machine.video.ac_regs[0x10] &= !0x08;
            }
            sys.machine.video.dirty = true;
        }
        0x07 => {
            // читать палитровый регистр: BL=регистр -> BH
            let v = sys.machine.video.ac_regs[(bl & 0x1F) as usize];
            sys.cpu.registers.set_bh(v);
        }
        0x08 => sys.cpu.registers.set_bh(sys.machine.video.ac_regs[0x11]),
        0x09 => {
            // читать все регистры в ES:DX (17 байт)
            for i in 0..16u16 {
                sys.write_u8(
                    es,
                    dx.wrapping_add(i),
                    sys.machine.video.ac_regs[i as usize],
                );
            }
            sys.write_u8(es, dx.wrapping_add(16), sys.machine.video.ac_regs[0x11]);
        }
        0x10 => {
            // установить цвет DAC: BX=индекс, CH=R, DH=G, CL=B (6 бит)
            let i = (bx & 0xFF) as usize;
            sys.machine.video.dac[i] = [
                sys.cpu.registers.ch() & 0x3F,
                sys.cpu.registers.dh() & 0x3F,
                sys.cpu.registers.cl() & 0x3F,
            ];
            sys.machine.video.dirty = true;
        }
        0x12 => {
            // блок цветов DAC: ES:DX, CX=кол-во, BX=старт (R,G,B на цвет)
            for i in 0..cx as u16 {
                let base = dx.wrapping_add(i * 3);
                let idx = (bx.wrapping_add(i)) as usize & 0xFF;
                sys.machine.video.dac[idx] = [
                    sys.read_u8(es, base) & 0x3F,
                    sys.read_u8(es, base.wrapping_add(1)) & 0x3F,
                    sys.read_u8(es, base.wrapping_add(2)) & 0x3F,
                ];
            }
            sys.machine.video.dirty = true;
        }
        0x15 => {
            // читать цвет DAC: BX=индекс -> CH=R, DH=G, CL=B
            let d = sys.machine.video.dac[(bx & 0xFF) as usize];
            sys.cpu.registers.set_ch(d[0]);
            sys.cpu.registers.set_dh(d[1]);
            sys.cpu.registers.set_cl(d[2]);
        }
        0x17 => {
            // читать блок DAC в ES:DX
            for i in 0..cx as u16 {
                let base = dx.wrapping_add(i * 3);
                let d = sys.machine.video.dac[(bx.wrapping_add(i)) as usize & 0xFF];
                sys.write_u8(es, base, d[0]);
                sys.write_u8(es, base.wrapping_add(1), d[1]);
                sys.write_u8(es, base.wrapping_add(2), d[2]);
            }
        }
        _ => log::info!("Unsupported INT 10h / AH=10h AL={:02X}", sub),
    }
}
