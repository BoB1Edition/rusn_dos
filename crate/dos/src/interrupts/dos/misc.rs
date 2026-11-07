// Ver: 5 File: crate/dos/src/interrupts/dos/misc.rs
use bus::Machine;
use x86::flags;

use crate::system::DosSystem;

pub fn set_carry_flag(sys: &mut DosSystem, value: bool) {
    let mut f = sys.cpu.registers.flags();
    if value {
        f |= flags::CF;
    } else {
        f &= !flags::CF;
    }
    sys.cpu.registers.set_flags(f);
}

pub fn set_dta(sys: &mut DosSystem) {
    sys.dta_segment = sys.cpu.registers.ds();
    sys.dta_offset = sys.cpu.registers.dx();
    set_carry_flag(sys, false);
    log::info!(
        "INT 21h / AH=1Ah: Set DTA -> {:04X}:{:04X}",
        sys.dta_segment,
        sys.dta_offset
    );
}

pub fn get_interrupt_vector(sys: &mut DosSystem) {
    let vector = sys.cpu.registers.al();
    let addr = (vector as u32) * 4;
    let ip = sys.machine.read_mem_u16(addr);
    let cs = sys.machine.read_mem_u16(addr + 2);
    sys.cpu.registers.set_es(cs);
    sys.cpu.registers.set_bx(ip);
    set_carry_flag(sys, false);
    log::info!(
        "INT 21h / AH=35h: Get interrupt vector {:02X}h -> {:04X}:{:04X}",
        vector,
        cs,
        ip
    );
}

pub fn set_interrupt_vector(sys: &mut DosSystem) {
    let vector = sys.cpu.registers.al();
    let handler_ip = sys.cpu.registers.dx();
    let handler_cs = sys.cpu.registers.ds();

    let addr = (vector as u32) * 4;
    sys.machine.write_mem_u16(addr, handler_ip);
    sys.machine.write_mem_u16(addr + 2, handler_cs);

    log::info!(
        "INT 21h / AH=25h: Set interrupt vector {:02X}h -> {:04X}:{:04X}",
        vector,
        handler_cs,
        handler_ip
    );
}

pub fn ioctl(sys: &mut DosSystem) {
    let al = sys.cpu.registers.al();
    let bx = sys.cpu.registers.bx();

    match al {
        0x00 => {
            if sys.filesystem.device_of(bx).is_some() {
                sys.cpu.registers.set_dx(0x80D3);
                set_carry_flag(sys, false);
            } else {
                let drive = sys.filesystem.get_current_drive();
                let drive_num = (drive as u8 - b'A') as u16;
                sys.cpu.registers.set_dx(drive_num);
                set_carry_flag(sys, false);
            }
        }
        _ => {
            log::warn!("Unsupported IOCTL subfunction AL={:02X}", al);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(0x0001);
        }
    }
}

pub fn get_country_info(sys: &mut DosSystem) {
    let dx = sys.cpu.registers.dx();

    // Подфункция SET country code (DX = 0xFFFF)
    if dx == 0xFFFF {
        log::info!(
            "INT 21h / AH=38h: Set country code to {} (ignored)",
            sys.cpu.registers.al()
        );
        sys.cpu.registers.set_ax(1); // возвращаем текущий country code = USA
        set_carry_flag(sys, false);
        return;
    }

    // Подфункция GET country info: DS:DX -> буфер (32..34 байта)
    let buf_seg = sys.cpu.registers.ds();
    let buf_off = dx;
    log::info!(
        "INT 21h / AH=38h: Get country info -> {:04X}:{:04X}",
        buf_seg,
        buf_off
    );

    // === Структура Country Information (US-стиль) ===
    // Offset 00h (WORD): Date format — 0=MDY, 1=DMY, 2=YMD
    sys.write_u16(buf_seg, buf_off + 0x00, 0);

    // Offset 02h (5 bytes ASCIIZ): Currency symbol — "$\0\0\0\0"
    sys.write_u8(buf_seg, buf_off + 0x02, b'$');
    sys.write_u8(buf_seg, buf_off + 0x03, 0);
    sys.write_u8(buf_seg, buf_off + 0x04, 0);
    sys.write_u8(buf_seg, buf_off + 0x05, 0);
    sys.write_u8(buf_seg, buf_off + 0x06, 0);

    // Offset 07h (2 bytes ASCIIZ): Thousands separator — ",\0"
    sys.write_u8(buf_seg, buf_off + 0x07, b',');
    sys.write_u8(buf_seg, buf_off + 0x08, 0);

    // Offset 09h (2 bytes ASCIIZ): Decimal separator — ".\0"
    sys.write_u8(buf_seg, buf_off + 0x09, b'.');
    sys.write_u8(buf_seg, buf_off + 0x0A, 0);

    // Offset 0Bh (2 bytes ASCIIZ): Date separator — "/\0"
    sys.write_u8(buf_seg, buf_off + 0x0B, b'/');
    sys.write_u8(buf_seg, buf_off + 0x0C, 0);

    // Offset 0Dh (2 bytes ASCIIZ): Time separator — ":\0"
    sys.write_u8(buf_seg, buf_off + 0x0D, b':');
    sys.write_u8(buf_seg, buf_off + 0x0E, 0);

    // Offset 0Fh (BYTE): Currency format flags
    //   bit 0: 0=$ before number, 1=$ after
    //   bit 1: 0=no space, 1=space between $ and number
    //   bit 2: 0=2 decimal places, 1=international default
    sys.write_u8(buf_seg, buf_off + 0x0F, 0x00);

    // Offset 10h (BYTE): Digits after decimal in currency
    sys.write_u8(buf_seg, buf_off + 0x10, 2);

    // Offset 11h (BYTE): Time format — 0=12h, 1=24h
    sys.write_u8(buf_seg, buf_off + 0x11, 0);

    // Offset 12h (DWORD): Far address of case-map routine (не используется)
    // Пишем двумя WORD'ами: IP, затем CS
    sys.write_u16(buf_seg, buf_off + 0x12, 0);
    sys.write_u16(buf_seg, buf_off + 0x14, 0);

    // Offset 16h (2 bytes ASCIIZ): Data list separator — ",\0"
    sys.write_u8(buf_seg, buf_off + 0x16, b',');
    sys.write_u8(buf_seg, buf_off + 0x17, 0);

    // Offset 18h (10 bytes): Reserved — zeros
    for i in 0..10 {
        sys.write_u8(buf_seg, buf_off + 0x18 + i, 0);
    }

    // AX = country code (1 = USA)
    sys.cpu.registers.set_ax(1);
    set_carry_flag(sys, false);
}

use std::time::{SystemTime, UNIX_EPOCH};

/// INT 21h / AH=2Ah — Get System Date
pub fn get_system_date(sys: &mut DosSystem) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let days = secs / 86400;
    // 1970-01-01 был четвергом (4). (days + 4) % 7 дает 0=Sun, 1=Mon...
    let day_of_week = ((days + 4) % 7) as u8;

    let mut year = 1970u16;
    let mut days_remaining = days;

    // Вычисляем год
    loop {
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if is_leap { 366 } else { 365 };
        if days_remaining < days_in_year {
            break;
        }
        days_remaining -= days_in_year;
        year += 1;
    }

    // Вычисляем месяц и день
    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let days_in_months = [
        31,
        if is_leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u8;
    for &dim in &days_in_months {
        if days_remaining < dim {
            break;
        }
        days_remaining -= dim;
        month += 1;
    }
    let day = (days_remaining + 1) as u8; // дни 1-based

    // Записываем в регистры
    sys.cpu.registers.set_cx(year);
    sys.cpu.registers.set_dh(month);
    sys.cpu.registers.set_dl(day);
    sys.cpu.registers.set_al(day_of_week);
    set_carry_flag(sys, false); // Успех, CF=0

    log::info!(
        "INT 21h / AH=2Ah: Get Date -> {:04}-{:02}-{:02} (dow={})",
        year,
        month,
        day,
        day_of_week
    );
}

pub fn get_system_time(sys: &mut DosSystem) {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let secs_of_day = secs % 86400;

    let hours = (secs_of_day / 3600) as u8;
    let minutes = ((secs_of_day % 3600) / 60) as u8;
    let seconds = (secs_of_day % 60) as u8;
    // Сотые доли из миллисекунд остатка (0..=99)
    let hundredths = (dur.subsec_millis() / 10) as u8;

    sys.cpu.registers.set_ch(hours);
    sys.cpu.registers.set_cl(minutes);
    sys.cpu.registers.set_dh(seconds);
    sys.cpu.registers.set_dl(hundredths);
    set_carry_flag(sys, false); // CF=0: функция всегда успешна

    log::info!(
        "INT 21h / AH=2Ch: Get Time -> {:02}:{:02}:{:02}.{:02}",
        hours,
        minutes,
        seconds,
        hundredths
    );
}

pub fn break_check_handler(sys: &mut DosSystem) {
    match sys.cpu.registers.al() {
        0x00 => {
            sys.cpu
                .registers
                .set_dl(if sys.break_check { 1 } else { 0 });
            log::info!(
                "INT 21h / AH=33h/AL=00: get break check -> {}",
                sys.break_check
            );
        }
        0x01 => {
            sys.break_check = sys.cpu.registers.dl() != 0;
            log::info!(
                "INT 21h / AH=33h/AL=01: set break check -> {}",
                sys.break_check
            );
        }
        0x02 => {
            let old = sys.break_check;
            sys.break_check = sys.cpu.registers.dl() != 0;
            sys.cpu.registers.set_dl(if old { 1 } else { 0 });
            log::info!(
                "INT 21h / AH=33h/AL=02: swap break check old={} new={}",
                old,
                sys.break_check
            );
        }
        0x05 => {
            sys.cpu.registers.set_dl(2);
            log::info!("INT 21h / AH=33h/AL=05: boot drive -> C: (2)");
        }
        0x06 => {
            sys.cpu.registers.set_bl(6); // major
            sys.cpu.registers.set_bh(0x16); // minor = 22
            sys.cpu.registers.set_dl(0); // revision
            sys.cpu.registers.set_dh(0);
            sys.cpu.registers.set_cx(0);
            log::info!("INT 21h / AH=33h/AL=06: true version -> 6.22");
        }
        _ => {
            log::warn!(
                "INT 21h / AH=33h: unsupported subfunction AL={:02X}",
                sys.cpu.registers.al()
            );
        }
    }
}

pub fn parse_filename(sys: &mut DosSystem) {
    let control = sys.cpu.registers.al();
    let skip_separators = (control & 0x01) != 0;
    let parse_drive = (control & 0x02) != 0;
    let parse_name = (control & 0x04) != 0;
    let parse_ext = (control & 0x08) != 0;

    let src_seg = sys.cpu.registers.ds();
    let mut src_off = sys.cpu.registers.si();
    let fcb_seg = sys.cpu.registers.es();
    let fcb_off = sys.cpu.registers.di();

    // Пропуск ведущих разделителей (бит 0)
    if skip_separators {
        loop {
            let b = sys.read_u8(src_seg, src_off);
            if is_leading_sep(b) {
                src_off = src_off.wrapping_add(1);
            } else {
                break;
            }
        }
    }

    let mut drive: u8 = 0;
    let mut name = [b' '; 8];
    let mut ext = [b' '; 3];
    let mut has_wildcard = false;

    if parse_drive {
        let c0 = sys.read_u8(src_seg, src_off);
        let c1 = sys.read_u8(src_seg, src_off.wrapping_add(1));
        if c1 == b':' && c0.is_ascii_alphabetic() {
            drive = c0.to_ascii_uppercase() - b'A' + 1;
            src_off = src_off.wrapping_add(2);
        }
    }
    if parse_name {
        let mut i = 0usize;
        loop {
            let b = sys.read_u8(src_seg, src_off);
            if b == 0 || is_parse_terminator(b) {
                break;
            }
            if b == b'*' {
                has_wildcard = true;
                while i < 8 {
                    name[i] = b'?';
                    i += 1;
                }
                src_off = src_off.wrapping_add(1);
                break;
            }
            if b == b'?' {
                has_wildcard = true;
            }
            if i < 8 {
                name[i] = b.to_ascii_uppercase();
                i += 1;
            }
            src_off = src_off.wrapping_add(1);
        }
    }

    if parse_ext {
        let b = sys.read_u8(src_seg, src_off);
        if b == b'.' {
            src_off = src_off.wrapping_add(1);
            let mut i = 0usize;
            loop {
                let b = sys.read_u8(src_seg, src_off);
                if b == 0 || is_parse_terminator(b) {
                    break;
                }
                if b == b'*' {
                    has_wildcard = true;
                    while i < 3 {
                        ext[i] = b'?';
                        i += 1;
                    }
                    src_off = src_off.wrapping_add(1);
                    break;
                }
                if b == b'?' {
                    has_wildcard = true;
                }
                if i < 3 {
                    ext[i] = b.to_ascii_uppercase();
                    i += 1;
                }
                src_off = src_off.wrapping_add(1);
            }
        }
    }

    sys.write_u8(fcb_seg, fcb_off, drive);
    for i in 0..8u16 {
        sys.write_u8(fcb_seg, fcb_off.wrapping_add(1 + i), name[i as usize]);
    }
    for i in 0..3u16 {
        sys.write_u8(fcb_seg, fcb_off.wrapping_add(9 + i), ext[i as usize]);
    }

    sys.cpu.registers.set_si(src_off);

    let result = if has_wildcard { 0x01u8 } else { 0x00u8 };
    sys.cpu.registers.set_al(result);
    set_carry_flag(sys, false);

    log::trace!(
        "INT21 AH=29 parse: drive={} name='{}' ext='{}' wildcard={} next_off={:04X}",
        drive,
        String::from_utf8_lossy(&name).trim_end(),
        String::from_utf8_lossy(&ext).trim_end(),
        has_wildcard,
        src_off
    );
}

fn is_parse_terminator(b: u8) -> bool {
    let result = matches!(
        b,
        0x00 | 0x09 | 0x0A | 0x0D | 0x20 |
        0x22 | 0x2B | 0x2C | 0x2E | 0x2F |
        0x3B | 0x3C | 0x3D | 0x3E |
        0x5B | 0x5C | 0x5D | 0x7C
    );
    result
}

fn is_leading_sep(b: u8) -> bool {
    let result = matches!(b, 0x09 | 0x20 | 0x2C | 0x3B);
    result
}
