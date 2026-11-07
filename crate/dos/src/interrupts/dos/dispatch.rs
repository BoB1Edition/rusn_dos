// Ver: 3 File: crate/dos/src/interrupts/dos/dispatch.rs
use crate::interrupts::dos::exec;
use crate::interrupts::dos::file_ops::file_date_time;
use crate::interrupts::dos::misc::{
    break_check_handler, get_country_info, get_system_date, get_system_time, parse_filename,
};
use crate::system::DosSystem;
use x86::flags;

use super::console::{
    buffered_input, char_input_no_echo, direct_char_input_no_echo, direct_console_io, print_char,
    print_dos_string, read_char_with_echo,
};
use super::dir::{get_current_directory, get_current_drive, set_current_directory};
use super::file_ops::{
    close_file, create_file, delete_file, file_attributes, open_file, read_file, seek_file,
    write_file,
};
use super::find::{find_first, find_next};
use super::memory::{allocate_memory_handler, free_memory_handler, modify_memory_handler};
use super::misc::{get_interrupt_vector, ioctl, set_carry_flag, set_dta, set_interrupt_vector};

pub fn handle_int21(sys: &mut DosSystem) {
    let ah = sys.cpu.registers.ah();
    let al = sys.cpu.registers.al();
    let bx = sys.cpu.registers.bx();
    let cx = sys.cpu.registers.cx();
    let dx = sys.cpu.registers.dx();
    let si = sys.cpu.registers.si();
    let di = sys.cpu.registers.di();
    let bp = sys.cpu.registers.bp();
    let ds = sys.cpu.registers.ds();
    let es = sys.cpu.registers.es();
    let cs = sys.cpu.registers.cs();
    let ip = sys.cpu.registers.ip();
    log::trace!(
        "INT21 AH={:02X} AL={:02X} BX={:04X} CX={:04X} DX={:04X} SI={:04X} DI={:04X} BP={:04X} DS={:04X} ES={:04X} CS:IP={:04X}:{:04X}",
        ah,
        al,
        bx,
        cx,
        dx,
        si,
        di,
        bp,
        ds,
        es,
        cs,
        ip
    );

    trace_int21_args(sys, ah, al, bx, cx, dx, ds, es);

    match ah {
        0x00 => {
            log::info!(
                "Program terminated via INT 21h AH=00h, exit code={}",
                sys.cpu.registers.al()
            );
            sys.exit_code = sys.cpu.registers.al();
            sys.program_terminated = true;
            sys.cpu.halted = true;
        }

        0x01 => read_char_with_echo(sys),
        0x02 => print_char(sys),
        0x06 => direct_console_io(sys),
        0x07 => direct_char_input_no_echo(sys),
        0x08 => char_input_no_echo(sys),
        0x09 => print_dos_string(sys),
        0x0A => buffered_input(sys),

        0x19 => get_current_drive(sys),
        0x1A => set_dta(sys),

        0x25 => set_interrupt_vector(sys),
        0x29 => parse_filename(sys),
        0x2A => get_system_date(sys),
        0x2C => get_system_time(sys),

        0x30 => {
            sys.cpu.registers.set_ax(0x1606);
            sys.cpu.registers.set_bx(0x0000);
            sys.cpu.registers.set_cx(0x0000);
            set_carry_flag(sys, false);
        }
        0x33 => break_check_handler(sys),
        0x35 => get_interrupt_vector(sys),
        0x38 => get_country_info(sys),
        0x3B => set_current_directory(sys),
        0x3C => create_file(sys),
        0x3D => open_file(sys),
        0x3E => close_file(sys),
        0x3F => read_file(sys),
        0x40 => write_file(sys),
        0x41 => delete_file(sys),
        0x42 => seek_file(sys),
        0x43 => file_attributes(sys),
        0x44 => ioctl(sys),
        0x47 => get_current_directory(sys),
        0x48 => allocate_memory_handler(sys),
        0x49 => free_memory_handler(sys),
        0x4A => modify_memory_handler(sys),
        0x4B => exec::exec_program(sys),

        0x4C => {
            log::debug!(
                "Program terminated via INT 21h AH=4Ch, exit code={}",
                sys.cpu.registers.al()
            );
            sys.exit_code = sys.cpu.registers.al();
            sys.program_terminated = true;
            sys.cpu.halted = true;
        }

        0x4D => exec::get_return_code(sys),
        0x4E => find_first(sys),
        0x4F => find_next(sys),
        0x50 => {
            // Set PSP
            sys.process_manager.current_psp = sys.cpu.registers.bx();
            log::trace!(
                "INT 21h / AH=50h: Set PSP to {:04X}",
                sys.process_manager.current_psp
            );
        }
        0x51 => {
            // Get PSP
            sys.cpu.registers.set_bx(sys.process_manager.current_psp);
            log::trace!(
                "INT 21h / AH=51h: Get PSP -> {:04X}",
                sys.process_manager.current_psp
            );
        }
        0x52 => {
            // Get List of Lists (SYSVARS)
            sys.cpu.registers.set_es(sys.sysvars_segment);
            sys.cpu.registers.set_bx(sys.sysvars_offset);
            log::trace!(
                "INT 21h / AH=52h: Get List of Lists -> {:04X}:{:04X}",
                sys.sysvars_segment,
                sys.sysvars_offset
            );
        }
        0x57 => file_date_time(sys),
        0x62 => {
            // Get PSP (дубликат 51h)
            sys.cpu.registers.set_bx(sys.process_manager.current_psp);
            log::trace!(
                "INT 21h / AH=62h: Get PSP -> {:04X}",
                sys.process_manager.current_psp
            );
        }
        _ => {
            log::warn!(
                "Unsupported DOS call 21h/{:02X}h at CS:IP={:04X}:{:04X}",
                sys.cpu.registers.ah(),
                sys.cpu.registers.cs(),
                sys.cpu.registers.ip()
            );
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(0x0001);
            sys.cpu.halted = true;
        }
    }

    let cf = flags::test_cf(sys.cpu.registers.flags());

    log::trace!(
        "INT21 AH={:02X} result CF={} AX={:04X} BX={:04X} CX={:04X} DX={:04X}",
        ah,
        if cf { 1 } else { 0 },
        sys.cpu.registers.ax(),
        sys.cpu.registers.bx(),
        sys.cpu.registers.cx(),
        sys.cpu.registers.dx()
    );
}

pub fn handle_int2f(sys: &mut DosSystem) {
    let ax = sys.cpu.registers.ax();
    let bx = sys.cpu.registers.bx();
    let cx = sys.cpu.registers.cx();
    let dx = sys.cpu.registers.dx();
    let cs = sys.cpu.registers.cs();
    let ip = sys.cpu.registers.ip();

    log::trace!(
        "INT2F AX={:04X} BX={:04X} CX={:04X} DX={:04X} CS:IP={:04X}:{:04X}",
        ax,
        bx,
        cx,
        dx,
        cs,
        ip
    );

    handle_int2f_inner(sys);

    let cf = flags::test_cf(sys.cpu.registers.flags());

    log::trace!(
        "INT2F result CF={} AX={:04X} BX={:04X} CX={:04X} DX={:04X}",
        if cf { 1 } else { 0 },
        sys.cpu.registers.ax(),
        sys.cpu.registers.bx(),
        sys.cpu.registers.cx(),
        sys.cpu.registers.dx()
    );
}

fn handle_int2f_inner(sys: &mut DosSystem) {
    let ax = sys.cpu.registers.ax();
    let ah = (ax >> 8) as u8;

    match ax {
        0x1687 => {
            if sys.machine_config.dpmi {
                let mut f = sys.cpu.registers.flags();
                f &= !flags::CF;
                sys.cpu.registers.set_flags(f);
                sys.cpu.registers.set_ax(0x0001);
                log::info!("INT 2Fh / AX=1687h: DPMI Host Check -> Installed (config)");
            } else {
                let mut f = sys.cpu.registers.flags();
                f |= flags::CF;
                sys.cpu.registers.set_flags(f);
                log::info!("INT 2Fh / AX=1687h: DPMI Host Check -> Not Installed (CF=1)");
            }
            return;
        }

        0x0C00 => {
            sys.cpu.registers.set_ax(0);
            return;
        }

        _ => {}
    }

    match ah {
        0x16 => {
            let mut f = sys.cpu.registers.flags();
            f |= flags::CF;
            sys.cpu.registers.set_flags(f);
        }
        0x4A => sys.cpu.registers.set_al(0),
        0x12 => sys.cpu.registers.set_al(0),
        0x0C => sys.cpu.registers.set_al(0),
        0x0D => sys.cpu.registers.set_al(0),
        _ => sys.cpu.registers.set_al(0),
    }
}

/// Дополнительная трассировка аргументов для наиболее важных функций INT 21h.
fn trace_int21_args(sys: &DosSystem, ah: u8, al: u8, bx: u16, cx: u16, dx: u16, ds: u16, es: u16) {
    // ASCIIZ-пути в DS:DX для файловых функций и EXEC.
    if matches!(ah, 0x3B | 0x3C | 0x3D | 0x41 | 0x43 | 0x4B | 0x4E) {
        let path = read_asciiz(sys, ds, dx, 128);
        log::trace!("INT21 AH={:02X} path='{}'", ah, path);
    }

    // DOS-строка до '$' для AH=09h.
    if ah == 0x09 {
        let s = read_dollar_string(sys, ds, dx, 256);
        log::trace!("INT21 AH=09 string='{}'", s.escape_debug());
    }

    // Запись в stdout/stderr: логируем содержимое буфера.
    if ah == 0x40 && (bx == 1 || bx == 2) {
        let len = cx.min(256);
        let s = read_bytes_lossy(sys, ds, dx, len);
        log::trace!(
            "INT21 AH=40 write handle={} requested_bytes={} data='{}'",
            bx,
            cx,
            s.escape_debug()
        );
    }

    // Для EXEC дополнительно логируем параметрический блок и командный хвост.
    if ah == 0x4B && al == 0x00 {
        let env_seg = sys.read_u16(es, bx);
        let cmd_off = sys.read_u16(es, bx.wrapping_add(2));
        let cmd_seg = sys.read_u16(es, bx.wrapping_add(4));
        let fcb1_off = sys.read_u16(es, bx.wrapping_add(6));
        let fcb1_seg = sys.read_u16(es, bx.wrapping_add(8));
        let fcb2_off = sys.read_u16(es, bx.wrapping_add(0x0A));
        let fcb2_seg = sys.read_u16(es, bx.wrapping_add(0x0C));

        let tail_len = sys.read_u8(cmd_seg, cmd_off);
        let tail = read_bytes_lossy(
            sys,
            cmd_seg,
            cmd_off.wrapping_add(1),
            (tail_len as u16).min(126),
        );

        log::trace!(
            "INT21 AH=4B param env={:04X} cmd={:04X}:{:04X} fcb1={:04X}:{:04X} fcb2={:04X}:{:04X} tail_len={} tail='{}'",
            env_seg,
            cmd_seg,
            cmd_off,
            fcb1_seg,
            fcb1_off,
            fcb2_seg,
            fcb2_off,
            tail_len,
            tail.escape_debug()
        );
    }

    // Управление памятью: сразу видно, сколько параграфов просят/освобождают/меняют.
    if ah == 0x48 {
        log::trace!(
            "INT21 AH=48 allocate memory request BX={:04X} paragraphs",
            bx
        );
    }

    if ah == 0x49 {
        log::trace!("INT21 AH=49 free memory segment ES={:04X}", es);
    }

    if ah == 0x4A {
        log::trace!(
            "INT21 AH=4A modify memory segment ES={:04X} new_size BX={:04X} paragraphs",
            es,
            bx
        );
    }

    // Seek: режим, handle и смещение.
    if ah == 0x42 {
        let cx_val = cx;
        let dx_val = dx;
        let offset = ((cx_val as u32) << 16) | dx_val as u32;
        log::trace!(
            "INT21 AH=42 seek handle={} mode={} offset={}",
            bx,
            al,
            offset
        );
    }

    // Close file.
    if ah == 0x3E {
        log::trace!("INT21 AH=3E close handle={}", bx);
    }

    // Read file.
    if ah == 0x3F {
        log::trace!("INT21 AH=3F read handle={} bytes={}", bx, cx);
    }
}

/// Читает ASCIIZ-строку из памяти, максимум `max_len` байт.
fn read_asciiz(sys: &DosSystem, segment: u16, offset: u16, max_len: u16) -> String {
    let mut bytes = Vec::new();

    for i in 0..max_len {
        let b = sys.read_u8(segment, offset.wrapping_add(i));
        if b == 0 {
            break;
        }
        bytes.push(b);
    }

    String::from_utf8_lossy(&bytes).into_owned()
}

/// Читает DOS-строку, заканчивающуюся символом '$'.
fn read_dollar_string(sys: &DosSystem, segment: u16, offset: u16, max_len: u16) -> String {
    let mut bytes = Vec::new();

    for i in 0..max_len {
        let b = sys.read_u8(segment, offset.wrapping_add(i));
        if b == b'$' {
            break;
        }
        bytes.push(b);
    }

    String::from_utf8_lossy(&bytes).into_owned()
}

/// Читает произвольный буфер и превращает его в lossy-строку.
fn read_bytes_lossy(sys: &DosSystem, segment: u16, offset: u16, len: u16) -> String {
    let mut bytes = Vec::new();

    for i in 0..len {
        bytes.push(sys.read_u8(segment, offset.wrapping_add(i)));
    }

    String::from_utf8_lossy(&bytes).into_owned()
}
