//! Обработка прерываний DOS (INT 21h, INT 2Fh)
//! Содержит реализацию основных функций DOS API

use crate::{DosMachine, error::Result};
use log::{warn, error};
use std::io::Write;

/// Обработчик прерывания INT 21h (основное DOS API)
pub fn handle_int21(machine: &mut DosMachine) {
    match machine.registers.ah() {
        0x02 => print_char(machine),
        0x06 => direct_console_io(machine),
        0x09 => print_dos_string(machine),
        0x3D => open_file(machine),
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

/// AH=3Dh — открытие файла
fn open_file(machine: &mut DosMachine) {
    // AL = режим доступа (биты 0-2: 0=только чтение, 1=только запись, 2=чтение+запись)
    let access_mode = machine.registers.al() & 0x07;
    
    // DS:DX = указатель на строку с именем файла (заканчивается нулём)
    let mut addr = ((machine.registers.ds() as u32) << 4).wrapping_add(machine.registers.dx() as u32);
    let mut filename = String::new();
    
    loop {
        if addr >= machine.memory.len() as u32 {
            error!("Filename string exceeds memory bounds");
            break;
        }
        let byte = machine.memory.read_u8(addr);
        if byte == 0 {
            break;
        }
        filename.push(byte as char);
        addr = addr.wrapping_add(1);
    }
    
    warn!(
        "INT 21h / AH=3Dh: Open file '{}' (mode={}) — not fully implemented",
        filename, access_mode
    );
    
    // Возвращаем фиктивный дескриптор файла (5 = первый доступный после стандартных)
    // CF=0 (успех), AX=дескриптор
    let mut flags = machine.registers.flags();
    flags &= !(1 << 0); // CF = 0
    machine.registers.set_flags(flags);
    machine.registers.set_ax(5); // фиктивный дескриптор
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