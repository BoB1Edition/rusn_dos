// Ver: 2 File: ./libs/dos_core/src/interrupts/dos.rs

use crate::{DosMachine, flags, mcb};
use log::error;
use std::io::{Read, Write};

/// Обработчик прерывания INT 21h (основное DOS API)
pub fn handle_int21(machine: &mut DosMachine) {
    log::info!("int 21: {:#02x}", machine.registers.ah());
    match machine.registers.ah() {
        0x00 => {
            log::info!(
                "Program terminated via INT 21h AH=00h, exit code={}",
                machine.registers.al()
            );
            machine.halted = true;
        }
        0x01 => read_char_with_echo(machine),
        0x02 => print_char(machine),
        0x06 => direct_console_io(machine),
        0x09 => print_dos_string(machine),
        0x19 => get_current_drive(machine),
        0x25 => set_interrupt_vector(machine),
        0x30 => {
            // Get DOS Version (возвращаем 6.22)
            machine.registers.set_ax(0x1606); // AL=6, AH=22
            machine.registers.set_bx(0x0000); // BH=0, BL=0
            machine.registers.set_cx(0x0000); // серийный номер = 0
            set_carry_flag(machine, false);
        }
        0x35 => get_interrupt_vector(machine),
        0x3B => set_current_directory(machine), // CHDIR
        0x3C => create_file(machine),           // CREATE FILE
        0x3D => open_file(machine),
        0x3E => close_file(machine),
        0x3F => read_file(machine),  // Чтение из файла
        0x40 => write_file(machine), // Запись в файл
        0x41 => delete_file(machine),           // DELETE FILE
        0x42 => seek_file(machine),  // Перемещение указателя
        0x43 => file_attributes(machine),
        0x47 => get_current_directory(machine),
        0x48 => allocate_memory_handler(machine),
        0x49 => free_memory_handler(machine),
        0x4A => modify_memory_handler(machine),
        0x4C => machine.halted = true,
        0x4E => find_first(machine),            // FIND FIRST
        0x4F => find_next(machine),             // FIND NEXT
        _ => {
            log::warn!(
                "Unsupported DOS call 21h/{:02X}h at CS:IP={:04X}:{:04X}",
                machine.registers.ah(),
                machine.registers.cs(),
                machine.registers.ip()
            );
            set_carry_flag(machine, true);
            machine.registers.set_ax(0x0001);
            machine.halted = true
        }
    }
}

/// Обработчик прерывания INT 2Fh (мультиплексное прерывание)
pub fn handle_int2f(machine: &mut DosMachine) {
    let ah = (machine.registers.ax() >> 8) as u8;
    match ah {
        0x16 => machine.registers.set_ax(0x8001), // DPMI не поддерживается
        0x4A => machine.registers.set_al(0),      // Резидент не найден
        0x12 => machine.registers.set_al(0),      // Сеть не обнаружена
        0x0C => machine.registers.set_al(0),      // Мышь не обнаружена
        0x0D => machine.registers.set_al(0),      // Принтер не обнаружен
        _ => machine.registers.set_al(0),         // Функция не поддерживается
    }
}

// === Функции INT 21h ===

/// AH=02h — вывод символа из регистра DL
fn print_char(machine: &DosMachine) {
    let ch = machine.registers.dl() as char;
    print!("{}", ch);
    std::io::stdout().flush().ok();
}

/// AH=06h — прямой ввод/вывод консоли
fn direct_console_io(machine: &mut DosMachine) {
    let dl = machine.registers.dl();

    if dl == 0xFF {
        // Неблокирующий ввод — в неинтерактивном режиме всегда "нет ввода"
        machine.registers.set_al(0);
        let mut flags = machine.registers.flags();
        flags |= flags::ZF; // ZF = 1
        machine.registers.set_flags(flags);
    } else {
        // Вывод символа
        let ch = dl as char;
        print!("{}", ch);
        std::io::stdout().flush().ok();

        // Устанавливаем AL = DL (эхо символа)
        machine.registers.set_al(dl);

        // Сбрасываем флаг ZF
        let mut flags = machine.registers.flags();
        flags &= !(flags::ZF); // ZF = 0
        machine.registers.set_flags(flags);
    }
}

/// AH=09h — вывод строки до '$'
fn print_dos_string(machine: &DosMachine) {
    let mut addr =
        ((machine.registers.ds() as u32) << 4).wrapping_add(machine.registers.dx() as u32);
    let mut s = String::new();

    loop {
        if addr >= machine.memory.len() as u32 {
            error!("DOS string does not contain terminating '$'");
            return;
        }

        let byte = machine.memory.read_u8(addr);
        if byte == b'$' {
            break;
        }
        s.push(byte as char);
        addr = addr.wrapping_add(1);
    }

    println!("{}", s);
}

fn read_char_with_echo(machine: &mut DosMachine) {
    let mut buffer = [0u8; 1];
    std::io::stdin().read_exact(&mut buffer).ok(); // Блокирующее чтение
    let ch = buffer[0];

    // Эхо
    print!("{}", ch as char);
    std::io::stdout().flush().ok();

    machine.registers.set_al(ch);
    let mut flags = machine.registers.flags();
    flags &= !(flags::ZF); // ZF = 0
    machine.registers.set_flags(flags);
}

fn set_carry_flag(machine: &mut DosMachine, value: bool) {
    let mut flags = machine.registers.flags();
    if value {
        flags |= flags::CF; // CF = 1
    } else {
        flags &= !(flags::CF); // CF = 0
    }
    machine.registers.set_flags(flags);
}

/// AH=3Dh — открытие файла
fn open_file(machine: &mut DosMachine) {
    let filename = machine.filesystem.extract_filename(
        machine.registers.ds(),
        machine.registers.dx(),
        |seg, off| machine.read_u8(seg, off),
    );
    let access_mode = machine.registers.al() & 0x07;

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
    match machine.filesystem.open_file(&filename, access_mode) {
        Ok(handle) => {
            set_carry_flag(machine, false);
            machine.registers.set_ax(handle);
            log::info!("File opened successfully, handle={}", handle);
        }
        Err(e) => {
            log::error!("Open failed: {}", e);
            set_carry_flag(machine, true);
            machine.registers.set_ax(2); // File not found
        }
    }
}

fn close_file(machine: &mut DosMachine) {
    let handle = machine.registers.bx();
    log::info!("INT 21h / AH=3Eh: Close file handle {}", handle);

    match machine.filesystem.close_file(handle) {
        Ok(_) => {
            set_carry_flag(machine, false);
            log::info!("File closed successfully");
        }
        Err(e) => {
            log::warn!("Close failed: {}", e);
            set_carry_flag(machine, true);
            machine.registers.set_ax(6); // Invalid handle
        }
    }
}

fn read_file(machine: &mut DosMachine) {
    let handle = machine.registers.bx();
    let count = machine.registers.cx();
    let buffer_seg = machine.registers.ds();
    let buffer_off = machine.registers.dx();

    log::info!(
        "INT 21h / AH=3Fh: Read {} bytes from handle {} to DS:{:#04x}",
        count,
        handle,
        buffer_off
    );

    // Выделяем буфер для чтения
    let mut buffer = vec![0u8; count as usize];

    match machine.filesystem.read_file(handle, &mut buffer) {
        Ok(bytes_read) => {
            // Записываем прочитанные байты в память
            for (i, &byte) in buffer.iter().take(bytes_read as usize).enumerate() {
                machine.write_u8(buffer_seg, buffer_off.wrapping_add(i as u16), byte);
            }

            set_carry_flag(machine, false);
            machine.registers.set_ax(bytes_read);
            log::info!("Read {} bytes successfully", bytes_read);
        }
        Err(e) => {
            log::error!("Read failed: {}", e);
            set_carry_flag(machine, true);
            machine.registers.set_ax(1); // Generic error
        }
    }
}

fn write_file(machine: &mut DosMachine) {
    let handle = machine.registers.bx();
    let count = machine.registers.cx();
    let buffer_seg = machine.registers.ds();
    let buffer_off = machine.registers.dx();

    log::info!(
        "INT 21h / AH=40h: Write {} bytes to handle {} from DS:{:#04x}",
        count,
        handle,
        buffer_off
    );

    // Читаем данные из памяти
    let mut buffer = vec![0u8; count as usize];
    for i in 0..count as usize {
        buffer[i] = machine.read_u8(buffer_seg, buffer_off.wrapping_add(i as u16));
    }

    match machine.filesystem.write_file(handle, &buffer) {
        Ok(bytes_written) => {
            set_carry_flag(machine, false);
            machine.registers.set_ax(bytes_written);
            log::info!("Wrote {} bytes successfully", bytes_written);
        }
        Err(e) => {
            log::error!("Write failed: {}", e);
            set_carry_flag(machine, true);
            machine.registers.set_ax(1); // Generic error
        }
    }
}

fn seek_file(machine: &mut DosMachine) {
    let handle = machine.registers.bx();
    let offset_high = machine.registers.cx() as i32;
    let offset_low = machine.registers.dx() as i32;
    let origin = machine.registers.al();

    // Собираем 32-битное смещение (little-endian)
    let offset = ((offset_high as i32) << 16) | (offset_low as i32);

    log::info!(
        "INT 21h / AH=42h: Seek handle {} to offset {} (origin={})",
        handle,
        offset,
        match origin {
            0 => "start",
            1 => "current",
            2 => "end",
            _ => "unknown",
        }
    );

    match machine.filesystem.seek_file(handle, offset, origin) {
        Ok(new_pos) => {
            set_carry_flag(machine, false);
            machine.registers.set_dx((new_pos >> 16) as u16);
            machine.registers.set_ax((new_pos & 0xFFFF) as u16);
            log::info!("Seek successful, new position={}", new_pos);
        }
        Err(e) => {
            log::error!("Seek failed: {}", e);
            set_carry_flag(machine, true);
            machine.registers.set_ax(1); // Generic error
        }
    }
}

fn get_interrupt_vector(machine: &mut DosMachine) {
    let vector = machine.registers.al();
    let addr = (vector as u32) * 4;
    // Читаем IP и CS из IVT (физическая память 0x00000)
    let ip = machine.read_phys_u16(addr);
    let cs = machine.read_phys_u16(addr + 2);
    // Возвращаем адрес в ES:BX
    machine.registers.set_es(cs);
    machine.registers.set_bx(ip);
    // Сбрасываем CF = 0 (успех)
    set_carry_flag(machine, false);
    log::info!(
        "INT 21h / AH=35h: Get interrupt vector {:02X}h -> {:04X}:{:04X}",
        vector,
        cs,
        ip
    );
}

fn set_interrupt_vector(machine: &mut DosMachine) {
    let vector = machine.registers.al();
    let handler_ip = machine.registers.dx(); // IP из DX
    let handler_cs = machine.registers.ds(); // CS из DS

    let addr = (vector as u32) * 4;
    machine.write_phys_u16(addr, handler_ip);
    machine.write_phys_u16(addr + 2, handler_cs);

    log::info!(
        "INT 21h / AH=25h: Set interrupt vector {:02X}h -> {:04X}:{:04X}",
        vector,
        handler_cs,
        handler_ip
    );
    // CF не изменяется (функция всегда успешна)
}

/// AH=43h — получить/установить атрибуты файла
fn file_attributes(machine: &mut DosMachine) {
    let al = machine.registers.al();
    let filename = machine.filesystem.extract_filename(
        machine.registers.ds(),
        machine.registers.dx(),
        |seg, off| machine.read_u8(seg, off),
    );

    match al {
        0x00 => {
            // Получить атрибуты
            log::info!(
                "INT 21h / AH=43h, AL=00h: Get file attributes for '{}'",
                filename
            );
            match machine.filesystem.resolve_path(&filename) {
                Ok(path) if path.exists() => {
                    // Возвращаем атрибут "архивный" (бит 5) и CF=0
                    machine.registers.set_cx(0x20);
                    set_carry_flag(machine, false);
                }
                _ => {
                    set_carry_flag(machine, true);
                    machine.registers.set_ax(2); // Файл не найден
                }
            }
        }
        0x01 => {
            // Установить атрибуты
            let new_attrs = machine.registers.cx();
            log::info!(
                "INT 21h / AH=43h, AL=01h: Set file attributes for '{}' to {:#04x}",
                filename,
                new_attrs
            );
            match machine.filesystem.resolve_path(&filename) {
                Ok(path) if path.exists() => {
                    // Реальная установка атрибутов не поддерживается,
                    // но возвращаем успех
                    set_carry_flag(machine, false);
                }
                _ => {
                    set_carry_flag(machine, true);
                    machine.registers.set_ax(2);
                }
            }
        }
        _ => {
            log::warn!("INT 21h / AH=43h: Unsupported AL={:02x}", al);
            set_carry_flag(machine, true);
            machine.registers.set_ax(1); // Недопустимая функция
        }
    }
}

/// AH=47h — получить текущий каталог для указанного диска
fn get_current_directory(machine: &mut DosMachine) {
    let dl = machine.registers.dl(); // номер диска (0=текущий, 1=A:, 2=B:, 3=C: и т.д.)
    let drive_letter = if dl == 0 {
        'C'
    } else {
        (b'A' + dl - 1) as char
    };

    let dir = machine
        .filesystem
        .get_current_directory(drive_letter)
        .unwrap_or("")
        .to_string();

    let seg = machine.registers.ds();
    let mut off = machine.registers.si();
    for &byte in dir.as_bytes() {
        machine.write_u8(seg, off, byte);
        off = off.wrapping_add(1);
    }
    machine.write_u8(seg, off, 0);

    set_carry_flag(machine, false);
    log::info!(
        "INT 21h / AH=47h: Get current directory for drive {}: -> '{}'",
        drive_letter,
        dir
    );
}

fn get_current_drive(machine: &mut DosMachine) {
    machine.registers.set_al(2); // Диск C:
    log::info!("INT 21h / AH=19h: Get current drive -> C:");
}

fn allocate_memory_handler(machine: &mut DosMachine) {
    let paragraphs = machine.registers.bx();
    let first_seg = machine.first_mcb_segment;
    match mcb::allocate(machine, first_seg, paragraphs) {
        Some(segment) => {
            machine.registers.set_ax(segment);
            set_carry_flag(machine, false);
        }
        None => {
            let max = mcb::max_available(machine, first_seg);
            machine.registers.set_ax(0x08);
            machine.registers.set_bx(max);
            set_carry_flag(machine, true);
        }
    }
}

fn free_memory_handler(machine: &mut DosMachine) {
    let segment = machine.registers.es();
    match mcb::free(machine, machine.first_mcb_segment, segment) {
        Ok(()) => set_carry_flag(machine, false),
        Err(code) => {
            set_carry_flag(machine, true);
            machine.registers.set_ax(code);
        }
    }
}

fn modify_memory_handler(machine: &mut DosMachine) {
    let segment = machine.registers.es();
    let new_paragraphs = machine.registers.bx();
    match mcb::modify(machine, machine.first_mcb_segment, segment, new_paragraphs) {
        Ok(_) => set_carry_flag(machine, false),
        Err(max) => {
            set_carry_flag(machine, true);
            machine.registers.set_ax(0x08);
            machine.registers.set_bx(max);
        }
    }
}

/// AH=3Bh — CHDIR (Set Current Directory)
fn set_current_directory(machine: &mut DosMachine) {
    let path = machine.filesystem.extract_filename(
        machine.registers.ds(),
        machine.registers.dx(),
        |seg, off| machine.read_u8(seg, off),
    );
    
    // Парсим букву диска, если она есть (например, "C:\DIR")
    let (drive, dir_path) = if path.len() >= 2 && path.chars().nth(1) == Some(':') {
        (path.chars().next().unwrap().to_ascii_uppercase(), &path[2..])
    } else {
        (machine.filesystem.get_current_drive(), path.as_str())
    };

    match machine.filesystem.set_current_directory(drive, dir_path) {
        Ok(_) => set_carry_flag(machine, false),
        Err(e) => {
            log::error!("CHDIR failed: {}", e);
            set_carry_flag(machine, true);
            machine.registers.set_ax(3); // AX=3: Path not found
        }
    }
}

/// AH=3Ch — CREATE FILE
fn create_file(machine: &mut DosMachine) {
    let path = machine.filesystem.extract_filename(
        machine.registers.ds(),
        machine.registers.dx(),
        |seg, off| machine.read_u8(seg, off),
    );
    let attrs = machine.registers.cx();
    
    match machine.filesystem.create_file(&path, attrs as u16) {
        Ok(handle) => {
            set_carry_flag(machine, false);
            machine.registers.set_ax(handle);
        }
        Err(e) => {
            log::error!("Create file failed: {}", e);
            set_carry_flag(machine, true);
            machine.registers.set_ax(5); // AX=5: Access denied
        }
    }
}

/// AH=41h — DELETE FILE
fn delete_file(machine: &mut DosMachine) {
    let path = machine.filesystem.extract_filename(
        machine.registers.ds(),
        machine.registers.dx(),
        |seg, off| machine.read_u8(seg, off),
    );
    
    match machine.filesystem.delete_file(&path) {
        Ok(_) => set_carry_flag(machine, false),
        Err(e) => {
            log::error!("Delete file failed: {}", e);
            set_carry_flag(machine, true);
            machine.registers.set_ax(2); // AX=2: File not found
        }
    }
}

/// AH=4Eh — FIND FIRST
fn find_first(machine: &mut DosMachine) {
    let path = machine.filesystem.extract_filename(
        machine.registers.ds(),
        machine.registers.dx(),
        |seg, off| machine.read_u8(seg, off),
    );
    let _search_attrs = machine.registers.cx(); // Атрибуты для поиска (пока игнорируем)
    let dta_addr = get_dta_addr(machine);

    match machine.filesystem.find_first(&path, dta_addr) {
        Ok(Some(found)) => {
            write_dta(machine, dta_addr, &found);
            set_carry_flag(machine, false);
        }
        Ok(None) => {
            set_carry_flag(machine, true);
            machine.registers.set_ax(18); // AX=18: No more files
        }
        Err(e) => {
            log::error!("Find first failed: {}", e);
            set_carry_flag(machine, true);
            machine.registers.set_ax(2); // AX=2: File not found
        }
    }
}

/// AH=4Fh — FIND NEXT
fn find_next(machine: &mut DosMachine) {
    let dta_addr = get_dta_addr(machine);
    
    match machine.filesystem.find_next(dta_addr) {
        Ok(Some(found)) => {
            write_dta(machine, dta_addr, &found);
            set_carry_flag(machine, false);
        }
        Ok(None) => {
            set_carry_flag(machine, true);
            machine.registers.set_ax(18); // AX=18: No more files
        }
        Err(e) => {
            log::error!("Find next failed: {}", e);
            set_carry_flag(machine, true);
            machine.registers.set_ax(18);
        }
    }
}

/// Возвращает физический адрес DTA (Disk Transfer Area).
/// По умолчанию в DOS DTA находится в PSP по смещению 0080h.
fn get_dta_addr(machine: &DosMachine) -> u32 {
    // В полноценном эмуляторе здесь нужно брать сегмент из AH=1Ah, 
    // но пока используем стандартный DS:0080h
    ((machine.registers.ds() as u32) << 4) + 0x0080
}

/// Записывает структуру FoundFile в DTA по стандарту DOS (43 байта).
fn write_dta(machine: &mut DosMachine, dta_addr: u32, found: &crate::filesystem::FoundFile) {
    // 0x00 - 0x14: Search context (21 байт). 
    // Наш filesystem.rs хранит контекст в HashMap по ключу dta_addr, 
    // поэтому эти 21 байт в памяти можем оставить нетронутыми.
    
    // 0x15: Атрибуты файла
    machine.write_phys_u8(dta_addr + 0x15, found.attr);
    // 0x16-0x17: Время
    machine.write_phys_u16(dta_addr + 0x16, found.time);
    // 0x18-0x19: Дата
    machine.write_phys_u16(dta_addr + 0x18, found.date);
    // 0x1A-0x1D: Размер файла (32 бита)
    machine.write_phys_u32(dta_addr + 0x1A, found.size);
    
    // 0x1E-0x2A: Имя файла в формате 8.3 (13 байт, включая null-terminator)
    let name_bytes = found.name_83.as_bytes();
    for (i, &b) in name_bytes.iter().take(12).enumerate() {
        machine.write_phys_u8(dta_addr + 0x1E + i as u32, b);
    }
    // Null-terminator
    machine.write_phys_u8(dta_addr + 0x1E + name_bytes.len().min(12) as u32, 0);
}