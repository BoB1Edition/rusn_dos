// Ver: 1 File: ./libs/dos_core/src/interrupts/dos.rs
//! Обработка прерываний DOS (INT 21h, INT 2Fh)
//! Содержит реализацию основных функций DOS API

use crate::{DosMachine, flags};
use log::error;
use std::io::{Read, Write};

/// Обработчик прерывания INT 21h (основное DOS API)
pub fn handle_int21(machine: &mut DosMachine) {
    log::debug!("int 21");
    log::info!("int 21: {:#02x}", machine.registers.ah());
    match machine.registers.ah() {
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
        0x3D => open_file(machine),
        0x3E => close_file(machine),
        0x3F => read_file(machine),  // Чтение из файла
        0x40 => write_file(machine), // Запись в файл
        0x42 => seek_file(machine),  // Перемещение указателя
        0x43 => file_attributes(machine),
        0x47 => get_current_directory(machine),
        0x4A => adjust_memory_block(machine),
        0x4C => machine.halted = true,
        _ => panic!("Unsupported DOS call AH={:#02x}", machine.registers.ah()),
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

/// AH=4Ah — изменение размера блока памяти
fn adjust_memory_block(machine: &mut DosMachine) {
    let requested_paragraphs = machine.registers.bx();
    const MAX_CONVENTIONAL_MEMORY_PARAGRAPHS: u16 = 0xA000; // 640 КБ = 0xA000 параграфов

    if requested_paragraphs <= MAX_CONVENTIONAL_MEMORY_PARAGRAPHS {
        // Успех: сбрасываем флаг переноса (CF=0)
        let mut flags = machine.registers.flags();
        flags &= !(flags::CF); // CF = 0
        machine.registers.set_flags(flags);
    } else {
        // Ошибка: недостаточно памяти
        let mut flags = machine.registers.flags();
        flags |= flags::CF; // CF = 1
        machine.registers.set_flags(flags);
        machine.registers.set_ax(0x08); // Код ошибки: недостаточно памяти
        machine.registers.set_bx(0xA000);
    }
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

fn extract_filename(machine: &DosMachine) -> String {
    let ds = machine.registers.ds();
    let dx = machine.registers.dx();

    let mut bytes = Vec::new();
    for i in 0..255 {
        let byte = machine.read_u8(ds, dx.wrapping_add(i as u16));
        if byte == 0 {
            break;
        }
        bytes.push(byte);
    }

    String::from_utf8_lossy(&bytes).to_string()
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
    log::info!("INT 21h / AH=35h: Get interrupt vector {:02X}h -> {:04X}:{:04X}", vector, cs, ip);
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
            log::info!("INT 21h / AH=43h, AL=00h: Get file attributes for '{}'", filename);
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

    let dir = machine.filesystem.get_current_directory(drive_letter).unwrap_or("").to_string();;

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