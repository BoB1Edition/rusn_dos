// Ver: 2
//! Обработка прерываний DOS (INT 21h, INT 2Fh)
//! Содержит реализацию основных функций DOS API

use crate::{DosMachine, filesystem, machine};
use log::{error, info, warn};
use std::{fs::{self, File as StdFile}, io::{Read, Seek, SeekFrom, Write}};

/// Обработчик прерывания INT 21h (основное DOS API)
pub fn handle_int21(machine: &mut DosMachine) {
    log::info!("int 21");
    log::debug!("int 21: {}", machine.registers.ah());
    match machine.registers.ah() {
        0x01 => read_char_with_echo(machine),
        0x02 => print_char(machine),
        0x06 => direct_console_io(machine),
        0x09 => print_dos_string(machine),
        0x3D => open_file(machine),
        0x3E => close_file(machine),
        0x3F => read_file(machine),   // Чтение из файла
        0x40 => write_file(machine),  // Запись в файл
        0x42 => seek_file(machine),   // Перемещение указателя
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
        flags |= 1 << 6; // ZF = 1
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
        flags &= !(1 << 6); // ZF = 0
        machine.registers.set_flags(flags);
    }
}

/// AH=09h — вывод строки до '$'
fn print_dos_string(machine: &DosMachine) {
    let mut addr = ((machine.registers.ds() as u32) << 4).wrapping_add(machine.registers.dx() as u32);
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
        flags &= !(1 << 0); // CF = 0
        machine.registers.set_flags(flags);
    } else {
        // Ошибка: недостаточно памяти
        let mut flags = machine.registers.flags();
        flags |= 1 << 0; // CF = 1
        machine.registers.set_flags(flags);
        machine.registers.set_ax(0x08); // Код ошибки: недостаточно памяти
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
    flags &= !(1 << 6); // ZF = 0
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
        flags |= 1 << 0; // CF = 1
    } else {
        flags &= !(1 << 0); // CF = 0
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
    let offset_low = machine.registers.cx() as u16;
    let offset_high = machine.registers.dx() as u16;
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