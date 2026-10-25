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
