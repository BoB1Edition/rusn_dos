// Ver: 1 File: crate/dos/src/interrupts/dos.rs

use bus::Machine;
use std::io::{Read, Write};
use x86::flags;

use crate::filesystem::FoundFile;
use crate::mcb;
use crate::system::DosSystem;

/// Обработчик прерывания INT 21h (основное DOS API)
pub fn handle_int21(sys: &mut DosSystem) {
    log::info!("int 21: {:#02x}", sys.cpu.registers.ah());
    match sys.cpu.registers.ah() {
        0x00 => {
            log::info!(
                "Program terminated via INT 21h AH=00h, exit code={}",
                sys.cpu.registers.al()
            );
            sys.cpu.halted = true;
        }
        0x01 => read_char_with_echo(sys),
        0x02 => print_char(sys),
        0x06 => direct_console_io(sys),
        0x09 => print_dos_string(sys),
        0x19 => get_current_drive(sys),
        0x25 => set_interrupt_vector(sys),
        0x30 => {
            // Get DOS Version (возвращаем 6.22)
            sys.cpu.registers.set_ax(0x1606); // AL=6, AH=22
            sys.cpu.registers.set_bx(0x0000);
            sys.cpu.registers.set_cx(0x0000);
            set_carry_flag(sys, false);
        }
        0x35 => get_interrupt_vector(sys),
        0x3B => set_current_directory(sys), // CHDIR
        0x3C => create_file(sys),           // CREATE FILE
        0x3D => open_file(sys),
        0x3E => close_file(sys),
        0x3F => read_file(sys),  // Чтение из файла
        0x40 => write_file(sys), // Запись в файл
        0x41 => delete_file(sys), // DELETE FILE
        0x42 => seek_file(sys),  // Перемещение указателя
        0x43 => file_attributes(sys),
        0x47 => get_current_directory(sys),
        0x48 => allocate_memory_handler(sys),
        0x49 => free_memory_handler(sys),
        0x4A => modify_memory_handler(sys),
        0x4C => sys.cpu.halted = true,
        0x4E => find_first(sys), // FIND FIRST
        0x4F => find_next(sys),  // FIND NEXT
        _ => {
            log::warn!(
                "Unsupported DOS call 21h/{:02X}h at CS:IP={:04X}:{:04X}",
                sys.cpu.registers.ah(),
                sys.cpu.registers.cs(),
                sys.cpu.registers.ip()
            );
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(0x0001);
            sys.cpu.halted = true
        }
    }
}

/// Обработчик прерывания INT 2Fh (мультиплексное прерывание)
pub fn handle_int2f(sys: &mut DosSystem) {
    let ah = (sys.cpu.registers.ax() >> 8) as u8;
    match ah {
        0x16 => sys.cpu.registers.set_ax(0x8001), // DPMI не поддерживается
        0x4A => sys.cpu.registers.set_al(0),      // Резидент не найден
        0x12 => sys.cpu.registers.set_al(0),      // Сеть не обнаружена
        0x0C => sys.cpu.registers.set_al(0),      // Мышь не обнаружена
        0x0D => sys.cpu.registers.set_al(0),      // Принтер не обнаружен
        _ => sys.cpu.registers.set_al(0),         // Функция не поддерживается
    }
}

// === Функции INT 21h ===

/// AH=02h — вывод символа из регистра DL
fn print_char(sys: &DosSystem) {
    let ch = sys.cpu.registers.dl() as char;
    print!("{}", ch);
    std::io::stdout().flush().ok();
}

/// AH=06h — прямой ввод/вывод консоли
fn direct_console_io(sys: &mut DosSystem) {
    let dl = sys.cpu.registers.dl();

    if dl == 0xFF {
        sys.cpu.registers.set_al(0);
        let mut f = sys.cpu.registers.flags();
        f |= flags::ZF; // ZF = 1
        sys.cpu.registers.set_flags(f);
    } else {
        print!("{}", dl as char);
        std::io::stdout().flush().ok();
        sys.cpu.registers.set_al(dl);
        let mut f = sys.cpu.registers.flags();
        f &= !flags::ZF;
        sys.cpu.registers.set_flags(f);
    }
}

/// AH=09h — вывод строки до '$'
fn print_dos_string(sys: &DosSystem) {
    let mut addr = DosSystem::phys(sys.cpu.registers.ds(), sys.cpu.registers.dx());
    let mut s = String::new();

    loop {
        let byte = sys.machine.read_mem_u8(addr);
        if byte == b'$' {
            break;
        }
        s.push(byte as char);
        addr = addr.wrapping_add(1);
    }

    println!("{}", s);
}

fn read_char_with_echo(sys: &mut DosSystem) {
    let mut buffer = [0u8; 1];
    std::io::stdin().read_exact(&mut buffer).ok();
    let ch = buffer[0];

    print!("{}", ch as char);
    std::io::stdout().flush().ok();

    sys.cpu.registers.set_al(ch);
    let mut f = sys.cpu.registers.flags();
    f &= !flags::ZF;
    sys.cpu.registers.set_flags(f);
}

fn set_carry_flag(sys: &mut DosSystem, value: bool) {
    let mut f = sys.cpu.registers.flags();
    if value {
        f |= flags::CF;
    } else {
        f &= !flags::CF;
    }
    sys.cpu.registers.set_flags(f);
}

/// AH=3Dh — открытие файла
fn open_file(sys: &mut DosSystem) {
    let filename =
        sys.filesystem
            .extract_filename(sys.cpu.registers.ds(), sys.cpu.registers.dx(), &sys.machine);
    let access_mode = sys.cpu.registers.al() & 0x07;

    log::info!(
        "INT 21h / AH=3Dh: Open file '{}' (mode={})",
        filename,
        match access_mode {
            0 => "read-only",
            1 => "write-only",
            2 => "read-write",
            _ => "unknown",
        }
    );
    match sys.filesystem.open_file(&filename, access_mode) {
        Ok(handle) => {
            set_carry_flag(sys, false);
            sys.cpu.registers.set_ax(handle);
            log::info!("File opened successfully, handle={}", handle);
        }
        Err(e) => {
            log::error!("Open failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(2); // File not found
        }
    }
}

fn close_file(sys: &mut DosSystem) {
    let handle = sys.cpu.registers.bx();
    log::info!("INT 21h / AH=3Eh: Close file handle {}", handle);

    match sys.filesystem.close_file(handle) {
        Ok(_) => {
            set_carry_flag(sys, false);
        }
        Err(e) => {
            log::warn!("Close failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(6); // Invalid handle
        }
    }
}

fn read_file(sys: &mut DosSystem) {
    let handle = sys.cpu.registers.bx();
    let count = sys.cpu.registers.cx();
    let buffer_seg = sys.cpu.registers.ds();
    let buffer_off = sys.cpu.registers.dx();

    let mut buffer = vec![0u8; count as usize];

    match sys.filesystem.read_file(handle, &mut buffer) {
        Ok(bytes_read) => {
            for (i, &byte) in buffer.iter().take(bytes_read as usize).enumerate() {
                sys.write_u8(buffer_seg, buffer_off.wrapping_add(i as u16), byte);
            }
            set_carry_flag(sys, false);
            sys.cpu.registers.set_ax(bytes_read);
        }
        Err(e) => {
            log::error!("Read failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(1);
        }
    }
}

fn write_file(sys: &mut DosSystem) {
    let handle = sys.cpu.registers.bx();
    let count = sys.cpu.registers.cx();
    let buffer_seg = sys.cpu.registers.ds();
    let buffer_off = sys.cpu.registers.dx();

    let mut buffer = vec![0u8; count as usize];
    for i in 0..count as usize {
        buffer[i] = sys.read_u8(buffer_seg, buffer_off.wrapping_add(i as u16));
    }

    match sys.filesystem.write_file(handle, &buffer) {
        Ok(bytes_written) => {
            set_carry_flag(sys, false);
            sys.cpu.registers.set_ax(bytes_written);
        }
        Err(e) => {
            log::error!("Write failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(1);
        }
    }
}

fn seek_file(sys: &mut DosSystem) {
    let handle = sys.cpu.registers.bx();
    let offset_high = sys.cpu.registers.cx() as i32;
    let offset_low = sys.cpu.registers.dx() as i32;
    let origin = sys.cpu.registers.al();

    let offset = ((offset_high as i32) << 16) | (offset_low as i32);

    match sys.filesystem.seek_file(handle, offset, origin) {
        Ok(new_pos) => {
            set_carry_flag(sys, false);
            sys.cpu.registers.set_dx((new_pos >> 16) as u16);
            sys.cpu.registers.set_ax((new_pos & 0xFFFF) as u16);
        }
        Err(e) => {
            log::error!("Seek failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(1);
        }
    }
}

fn get_interrupt_vector(sys: &mut DosSystem) {
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

fn set_interrupt_vector(sys: &mut DosSystem) {
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

/// AH=43h — получить/установить атрибуты файла
fn file_attributes(sys: &mut DosSystem) {
    let al = sys.cpu.registers.al();
    let filename =
        sys.filesystem
            .extract_filename(sys.cpu.registers.ds(), sys.cpu.registers.dx(), &sys.machine);

    match al {
        0x00 => match sys.filesystem.resolve_path(&filename) {
            Ok(path) if path.exists() => {
                sys.cpu.registers.set_cx(0x20);
                set_carry_flag(sys, false);
            }
            _ => {
                set_carry_flag(sys, true);
                sys.cpu.registers.set_ax(2);
            }
        },
        0x01 => match sys.filesystem.resolve_path(&filename) {
            Ok(path) if path.exists() => {
                set_carry_flag(sys, false);
            }
            _ => {
                set_carry_flag(sys, true);
                sys.cpu.registers.set_ax(2);
            }
        },
        _ => {
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(1);
        }
    }
}

/// AH=47h — получить текущий каталог для указанного диска
fn get_current_directory(sys: &mut DosSystem) {
    let dl = sys.cpu.registers.dl();
    let drive_letter = if dl == 0 {
        'C'
    } else {
        (b'A' + dl - 1) as char
    };

    let dir = sys
        .filesystem
        .get_current_directory(drive_letter)
        .unwrap_or("")
        .to_string();

    let seg = sys.cpu.registers.ds();
    let mut off = sys.cpu.registers.si();
    for &byte in dir.as_bytes() {
        sys.write_u8(seg, off, byte);
        off = off.wrapping_add(1);
    }
    sys.write_u8(seg, off, 0);

    set_carry_flag(sys, false);
    log::info!(
        "INT 21h / AH=47h: Get current directory for drive {}: -> '{}'",
        drive_letter,
        dir
    );
}

fn get_current_drive(sys: &mut DosSystem) {
    sys.cpu.registers.set_al(2); // Диск C:
    log::info!("INT 21h / AH=19h: Get current drive -> C:");
}

fn allocate_memory_handler(sys: &mut DosSystem) {
    let paragraphs = sys.cpu.registers.bx();
    let first = sys.first_mcb_segment;
    match mcb::allocate(&mut sys.machine, first, paragraphs) {
        Some(segment) => {
            sys.cpu.registers.set_ax(segment);
            set_carry_flag(sys, false);
        }
        None => {
            let max = mcb::max_available(&sys.machine, first);
            sys.cpu.registers.set_ax(0x08);
            sys.cpu.registers.set_bx(max);
            set_carry_flag(sys, true);
        }
    }
}

fn free_memory_handler(sys: &mut DosSystem) {
    let segment = sys.cpu.registers.es();
    let first = sys.first_mcb_segment;
    match mcb::free(&mut sys.machine, first, segment) {
        Ok(()) => set_carry_flag(sys, false),
        Err(code) => {
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(code);
        }
    }
}

fn modify_memory_handler(sys: &mut DosSystem) {
    let segment = sys.cpu.registers.es();
    let new_paragraphs = sys.cpu.registers.bx();
    let first = sys.first_mcb_segment;
    match mcb::modify(&mut sys.machine, first, segment, new_paragraphs) {
        Ok(_) => set_carry_flag(sys, false),
        Err(max) => {
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(0x08);
            sys.cpu.registers.set_bx(max);
        }
    }
}

/// AH=3Bh — CHDIR (Set Current Directory)
fn set_current_directory(sys: &mut DosSystem) {
    let path =
        sys.filesystem
            .extract_filename(sys.cpu.registers.ds(), sys.cpu.registers.dx(), &sys.machine);

    let (drive, dir_path) = if path.len() >= 2 && path.chars().nth(1) == Some(':') {
        (
            path.chars().next().unwrap().to_ascii_uppercase(),
            &path[2..],
        )
    } else {
        (sys.filesystem.get_current_drive(), path.as_str())
    };

    match sys.filesystem.set_current_directory(drive, dir_path) {
        Ok(_) => set_carry_flag(sys, false),
        Err(e) => {
            log::error!("CHDIR failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(3); // AX=3: Path not found
        }
    }
}

/// AH=3Ch — CREATE FILE
fn create_file(sys: &mut DosSystem) {
    let path =
        sys.filesystem
            .extract_filename(sys.cpu.registers.ds(), sys.cpu.registers.dx(), &sys.machine);
    let attrs = sys.cpu.registers.cx();

    match sys.filesystem.create_file(&path, attrs) {
        Ok(handle) => {
            set_carry_flag(sys, false);
            sys.cpu.registers.set_ax(handle);
        }
        Err(e) => {
            log::error!("Create file failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(5); // AX=5: Access denied
        }
    }
}

/// AH=41h — DELETE FILE
fn delete_file(sys: &mut DosSystem) {
    let path =
        sys.filesystem
            .extract_filename(sys.cpu.registers.ds(), sys.cpu.registers.dx(), &sys.machine);

    match sys.filesystem.delete_file(&path) {
        Ok(_) => set_carry_flag(sys, false),
        Err(e) => {
            log::error!("Delete file failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(2); // AX=2: File not found
        }
    }
}

/// AH=4Eh — FIND FIRST
fn find_first(sys: &mut DosSystem) {
    let path =
        sys.filesystem
            .extract_filename(sys.cpu.registers.ds(), sys.cpu.registers.dx(), &sys.machine);
    let dta_addr = get_dta_addr(sys);

    match sys.filesystem.find_first(&path, dta_addr) {
        Ok(Some(found)) => {
            write_dta(sys, dta_addr, &found);
            set_carry_flag(sys, false);
        }
        Ok(None) => {
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(18); // AX=18: No more files
        }
        Err(e) => {
            log::error!("Find first failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(2); // AX=2: File not found
        }
    }
}

/// AH=4Fh — FIND NEXT
fn find_next(sys: &mut DosSystem) {
    let dta_addr = get_dta_addr(sys);

    match sys.filesystem.find_next(dta_addr) {
        Ok(Some(found)) => {
            write_dta(sys, dta_addr, &found);
            set_carry_flag(sys, false);
        }
        Ok(None) => {
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(18);
        }
        Err(e) => {
            log::error!("Find next failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(18);
        }
    }
}

/// Возвращает физический адрес DTA (Disk Transfer Area).
/// По умолчанию в DOS DTA находится в PSP по смещению 0080h.
fn get_dta_addr(sys: &DosSystem) -> u32 {
    ((sys.cpu.registers.ds() as u32) << 4) + 0x0080
}

/// Записывает структуру FoundFile в DTA по стандарту DOS (43 байта).
fn write_dta(sys: &mut DosSystem, dta_addr: u32, found: &FoundFile) {
    sys.write_phys_u8(dta_addr + 0x15, found.attr);
    sys.write_phys_u16(dta_addr + 0x16, found.time);
    sys.write_phys_u16(dta_addr + 0x18, found.date);
    sys.write_phys_u32(dta_addr + 0x1A, found.size);

    let name_bytes = found.name_83.as_bytes();
    for (i, &b) in name_bytes.iter().take(12).enumerate() {
        sys.write_phys_u8(dta_addr + 0x1E + i as u32, b);
    }
    sys.write_phys_u8(dta_addr + 0x1E + name_bytes.len().min(12) as u32, 0);
}
