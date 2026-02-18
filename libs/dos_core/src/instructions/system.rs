// Ver: 4
use log::{error, warn};

use crate::{interrupts::bios, machine::DosMachine};

pub(crate) fn int(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let vector = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    bytes.push(vector);
    let _ = machine.log_instruction(csip, &bytes);
    machine.registers.step(None);
    match vector {
        0x21 => machine.handle_int21(),
        0x2F => machine.handle_int2f(),
        0x20 => machine.halted = true,
        0x10 => bios::handle_int10(machine),
        0x16 => bios::handle_int16(machine),
        _ => {
            error!("Unsupported interrupt: INT {:#02X}", vector);
            machine.halted = true;
        }
    }
}

pub fn in_al_dx(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    bytes.push(0xEC); // опкод IN AL, DX

    let port = machine.registers.dx();
    let value = match port {
        0x60 => {
            // Порт клавиатуры (8042) — возвращаем сканкод клавиши
            // Для демо-режима возвращаем '1' (сканкод 0x02) с периодической сменой
            let tick = machine.registers.ip() as u64 / 1000;
            match tick % 4 {
                0 => 0x02, // '1'
                1 => 0x03, // '2'
                2 => 0x04, // '3'
                _ => 0x01, // ESC (для выхода)
            }
        }
        0x40..=0x42 => {
            // Порты таймера 8253/8254 — возвращаем текущее время в мс
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            (now.as_millis() & 0xFF) as u8
        }
        0x20 | 0xA0 => {
            // PIC (Programmable Interrupt Controller) — всегда готов к обслуживанию
            0x00
        }
        0x3C2 | 0x3C4 | 0x3C5 | 0x3CE | 0x3CF => {
            // VGA контроллер — заглушка для совместимости
            0x00
        }
        _ => {
            warn!("IN AL, DX from unimplemented port {:#04x}", port);
            0x00 // заглушка для неизвестных портов
        }
    };

    machine.registers.set_al(value);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn nop(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let bytes = prev.to_vec();
    machine.log_instruction(csip, &bytes).ok();
}

pub fn hlt(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let bytes = prev.to_vec();

    log::info!(
        "HLT instruction executed at CS:IP={:#04x}:{:#04x}, halting CPU",
        machine.registers.cs(),
        machine.registers.ip()
    );
    machine.halted = true;

    machine.log_instruction(csip, &bytes).ok();
}

pub fn cmc(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let bytes = prev.to_vec();

    // Инвертируем флаг переноса (бит 0)
    let flags = machine.registers.flags();
    let new_flags = flags ^ 1; // XOR с 1 инвертирует младший бит (CF)
    machine.registers.set_flags(new_flags);

    machine.log_instruction(csip, &bytes).ok();
}

pub fn iret(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let bytes = prev.to_vec();

    // 1. Извлекаем IP из стека
    let sp = machine.registers.sp();
    let ip = machine.read_u16(machine.registers.ss(), sp);
    machine.registers.set_sp(sp.wrapping_add(2));

    // 2. Извлекаем CS из стека
    let sp = machine.registers.sp();
    let cs = machine.read_u16(machine.registers.ss(), sp);
    machine.registers.set_sp(sp.wrapping_add(2));

    // 3. Извлекаем FLAGS из стека
    let sp = machine.registers.sp();
    let flags = machine.read_u16(machine.registers.ss(), sp);
    machine.registers.set_sp(sp.wrapping_add(2));

    // Устанавливаем восстановленные значения
    machine.registers.set_ip(ip);
    machine.registers.set_cs(cs);
    machine.registers.set_flags(flags);

    // Логирование с указанием адреса возврата
    log::info!(
        "IRET: returning to {:#04x}:{:#04x}, flags={:#04x}",
        cs,
        ip,
        flags
    );

    machine.log_instruction(csip, &bytes).ok();
}

pub fn in_al_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let port = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(port);

    let value = match port {
        0x60 => {
            // Порт данных клавиатуры (8042)
            // Возвращаем сканкод с битом 7=0 (нажатие) для простоты
            let tick = machine.registers.ip() as u64 / 1000;
            match tick % 4 {
                0 => 0x1E, // 'A'
                1 => 0x30, // 'B'
                2 => 0x2E, // 'C'
                _ => 0x01, // ESC
            }
        }
        0x64 => {
            // Порт статуса/команд контроллера клавиатуры (8042)
            // Бит 0 (OBF): 1 = данные готовы в порту 0x60
            // Бит 1 (IBF): 0 = контроллер свободен для приёма команд
            // Бит 2 (SYS): 1 = система загружена (POST завершён)
            // Бит 3 (CMD/DATA): 0 = следующая запись в 0x60 будет данными
            // Биты 4-7: 0 = нет ошибок, нет мыши
            0x1C // 00011100b = биты 2,3,4 установлены (система готова, ожидаем данные)
        }
        0x40..=0x42 => {
            // Порты таймера 8253/8254
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            (now.as_millis() & 0xFF) as u8
        }
        0x20 | 0xA0 => 0x00, // PIC — всегда готов к обслуживанию
        0x92 => 0x02,        // Fast A20 Gate — линия A20 включена
        _ => {
            warn!("IN AL, imm8 from unimplemented port {:#02x}", port);
            0x00 // заглушка для неизвестных портов
        }
    };

    machine.registers.set_al(value);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn out_imm8_al(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let port = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(port);

    let value = machine.registers.al();

    match port {
        0x40..=0x42 => {
            // Порты данных таймера 8253/8254
            let channel = port - 0x40;
            log::info!(
                "OUT AL, {:#02x} to timer channel {} (port {:#02x})",
                value, channel, port
            );

            // Сохраняем значение счётчика для будущей эмуляции (опционально)
            match channel {
                0 => machine.timer_channel_0 = Some(value),
                1 => machine.timer_channel_1 = Some(value),
                2 => machine.timer_channel_2 = Some(value),
                _ => {}
            }
        }
        0x43 => {
            // Порт управления таймером 8253/8254
            let sc = (value >> 6) & 0x03;   // выбор канала (0-3)
            let rw = (value >> 4) & 0x03;   // режим доступа
            let mode = (value >> 1) & 0x07; // режим работы (0-5)
            let bcd = value & 0x01;         // BCD/двоичный режим
            
            // Декодируем для логирования
            let channel_name = match sc {
                0 => "Channel 0 (IRQ0/system timer)",
                1 => "Channel 1 (DRAM refresh)",
                2 => "Channel 2 (speaker)",
                3 => "Read-back command",
                _ => "Unknown",
            };
            
            let access_mode = match rw {
                0 => "latch count",
                1 => "low byte only",
                2 => "high byte only",
                3 => "low then high byte (16-bit)",
                _ => "unknown",
            };
            
            let operation_mode = match mode {
                0 => "interrupt on terminal count",
                1 => "one-shot",
                2 => "rate generator (periodic)",
                3 => "square wave generator",
                4 => "software triggered strobe",
                5 => "hardware triggered strobe",
                _ => "unknown",
            };
            
            log::info!(
                "Timer control: {} | {} | {} | BCD={}",
                channel_name, access_mode, operation_mode, bcd
            );
            
            // Особый случай: инициализация канала 0 для системного таймера
            if sc == 0 && mode == 2 && rw == 3 {
                log::info!("Timer channel 0 initialized for periodic interrupts (IRQ0)");
                machine.timer_initialized = true;
            }
        }
        0x61 => {
            // Порт динамика/клавиатуры (PC speaker control)
            log::info!("OUT AL, {:#02x} to speaker port 0x61", value);
            // Бит 0: включение/выключение динамика
            // Бит 1: включение/выключение генератора частоты
        }
        0x20 | 0xA0 => {
            // PIC (Programmable Interrupt Controller)
            if value == 0x20 {
                log::info!("OUT AL, 20h to PIC port {:#02x} (End of Interrupt)", port);
            } else {
                log::info!("OUT AL, {:#02x} to PIC port {:#02x}", value, port);
            }
        }
        0x60 => {
            // Порт данных клавиатуры (8042)
            // Обрабатываем команды контроллера клавиатуры
            match value {
                0xED => {
                    // Следующий байт будет содержать состояние светодиодов
                    // (бит 0 = Scroll Lock, бит 1 = Num Lock, бит 2 = Caps Lock)
                    log::info!("Keyboard command 0xED received (set LEDs pending)");
                    machine.keyboard_led_command_pending = true;
                }
                0xF4 => log::info!("Keyboard scanning enabled (command 0xF4)"),
                0xF5 => log::info!("Keyboard scanning disabled (command 0xF5)"),
                0xF6 => log::info!("Keyboard default parameters set (command 0xF6)"),
                0xFE => log::info!("Keyboard retransmit requested (command 0xFE)"),
                0xDF => {
                    // Нестандартная команда — часто используется для установки светодиодов
                    // Биты 0-2: состояние индикаторов (Scroll/Num/Caps Lock)
                    let scroll_lock = (value & 0x01) != 0;
                    let num_lock = (value & 0x02) != 0;
                    let caps_lock = (value & 0x04) != 0;
                    log::info!(
                        "Keyboard LEDs set via 0xDF: Scroll={}, Num={}, Caps={}",
                        scroll_lock,
                        num_lock,
                        caps_lock
                    );
                }
                _ => {
                    // Любая другая команда — игнорируем, но логируем для отладки
                    log::info!(
                        "Keyboard command {:#02x} written to port 0x60 (ignored)",
                        value
                    );
                }
            }
        }
        0x64 => {
            // Порт команд контроллера клавиатуры (8042)
            match value {
                0xAD => log::info!("Keyboard disabled via port 0x64"),
                0xAE => log::info!("Keyboard enabled via port 0x64"),
                0xFF => log::info!("Keyboard reset requested via port 0x64"),
                _ => log::info!("Keyboard command {:#02x} to port 0x64 (ignored)", value),
            }
        }
        0x92 => {
            // Fast A20 Gate / Keyboard Control
            if value & 0x80 != 0 {
                warn!("System reset requested via port 0x92 — halting CPU");
                machine.halted = true;
            } else {
                log::info!("A20 gate control: value={:#02x}", value);
            }
        }
        _ => {
            warn!(
                "OUT AL, imm8 to unimplemented port {:#02x} (value={:#02x})",
                port, value
            );
        }
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub fn std(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let bytes = prev.to_vec();

    // Устанавливаем флаг направления DF (бит 10) в 1
    let mut flags = machine.registers.flags();
    flags |= 1 << 10; // DF = 1
    machine.registers.set_flags(flags);

    machine.log_instruction(csip, &bytes).ok();
}

pub fn stc(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let bytes = prev.to_vec();

    // Устанавливаем флаг переноса CF (бит 0) в 1
    let mut flags = machine.registers.flags();
    flags |= 1; // CF = 1
    machine.registers.set_flags(flags);

    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/system.rs
pub fn clc(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let bytes = prev.to_vec();

    let mut flags = machine.registers.flags();
    flags &= !1; // CF = 0 (очищаем бит 0)
    machine.registers.set_flags(flags);

    machine.log_instruction(csip, &bytes).ok();
}

// libs/dos_core/src/instructions/system.rs
pub fn sahf(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    bytes.push(0x9E);

    let ah = machine.registers.ah() as u16;
    let mut flags = machine.registers.flags();

    // Маска для битов, которые обновляются: CF(0), PF(2), AF(4), ZF(6), SF(7)
    const MASK: u16 = 0x00D5; // 0000 0000 1101 0101

    // Очищаем целевые биты в флагах
    flags &= !MASK;

    // Копируем биты из AH в их позиции во флагах (без сдвига — позиции совпадают)
    flags |= ah & MASK;

    machine.registers.set_flags(flags);
    machine.log_instruction(csip, &bytes).ok();
}

pub fn outsw(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    bytes.push(0x6F); // опкод OUTSW
    
    // Определяем сегмент источника с учётом префикса
    let src_segment = machine.override_segment.unwrap_or(machine.registers.ds());
    
    // Читаем слово из источника [DS:SI] → AX
    let si = machine.registers.si();
    let word = machine.read_u16(src_segment, si);
    machine.registers.set_ax(word);
    
    // Выводим слово в порт DX (эмуляция)
    let port = machine.registers.dx();
    let value = word;
    
    // Эмуляция вывода слова в порт (реально требует двух операций OUT)
    match port {
        0x3F8..=0x3FF => {
            // COM1 последовательный порт — эмулируем вывод символа
            let char = (value & 0xFF) as u8;
            if char >= 32 && char < 127 {
                log::info!("OUTSW to COM1 (port {:#04x}): character '{}'", port, char as char);
            } else {
                log::info!("OUTSW to COM1 (port {:#04x}): byte {:#02x}", port, char);
            }
        }
        0x378..=0x37A => {
            // LPT1 параллельный порт (принтер)
            log::info!("OUTSW to LPT1 (port {:#04x}): value {:#04x}", port, value);
        }
        _ => {
            log::info!("OUTSW to port {:#04x}: value {:#04x} (ignored)", port, value);
        }
    }
    
    // Обновляем указатель в зависимости от флага направления DF (бит 10)
    let df = (machine.registers.flags() & (1 << 10)) != 0;
    if df {
        machine.registers.set_si(si.wrapping_sub(2));
    } else {
        machine.registers.set_si(si.wrapping_add(2));
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

/// OUTSD — Output String Doubleword (32-bit)
/// Выводит двойное слово из [DS:ESI] в порт DX, обновляет ESI на ±4
pub fn outsd(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    bytes.push(0x66); // префикс операнда
    bytes.push(0x6F); // опкод OUTSD
    
    // Определяем сегмент источника с учётом префикса
    let src_segment = machine.override_segment.unwrap_or(machine.registers.ds());
    
    // Читаем двойное слово из источника [DS:ESI] → EAX
    let esi = machine.registers.esi();
    let dword = machine.read_u32(src_segment, (esi & 0xFFFF) as u16); // усечение до 16 бит для реального режима
    machine.registers.set_eax(dword);
    
    // Выводим двойное слово в порт DX (эмуляция)
    let port = machine.registers.dx();
    let value = dword;
    
    log::info!("OUTSD to port {:#04x}: value {:#08x} (ignored in real mode)", port, value);
    
    // Обновляем указатель в зависимости от флага направления DF (бит 10)
    let df = (machine.registers.flags() & (1 << 10)) != 0;
    if df {
        machine.registers.set_esi(esi.wrapping_sub(4));
    } else {
        machine.registers.set_esi(esi.wrapping_add(4));
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn lahf(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    bytes.push(0x9F); // опкод LAHF
    
    // Читаем текущие флаги (тип u16)
    let flags = machine.registers.flags();
    
    // Извлекаем нужные биты и формируем значение для AH
    // Биты копируются в те же позиции (0→0, 2→2, 4→4, 6→6, 7→7)
    let ah = ((flags & 0x01) << 0)   // CF → бит 0
           | ((flags & 0x04) << 0)   // PF → бит 2 (маска 0x04 = бит 2)
           | ((flags & 0x10) << 0)   // AF → бит 4 (маска 0x10 = бит 4)
           | ((flags & 0x40) << 0)   // ZF → бит 6 (маска 0x40 = бит 6)
           | ((flags & 0x80) << 0);  // SF → бит 7 (маска 0x80 = бит 7)
    
    // Устанавливаем значение в регистр AH
    machine.registers.set_ah(ah as u8);
    
    // Флаги НЕ изменяются — критически важно!
    machine.log_instruction(csip, &bytes).ok();
}


pub fn outsb(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    bytes.push(0x6E); // опкод OUTSB
    
    // Определяем сегмент источника с учётом префикса
    let src_segment = machine.override_segment.unwrap_or(machine.registers.ds());
    
    // Читаем байт из источника [DS:SI] → AL
    let si = machine.registers.si();
    let byte = machine.read_u8(src_segment, si);
    machine.registers.set_al(byte);
    
    // Выводим байт в порт DX (эмуляция)
    let port = machine.registers.dx();
    let value = byte;
    
    // Эмуляция вывода в различные порты
    match port {
        0x60 => {
            // Порт данных клавиатуры (8042) — эмуляция команд
            log::info!("OUTSB to keyboard data port (0x60): command {:#02x}", value);
            if value == 0xED {
                machine.keyboard_led_command_pending = true;
            }
        }
        0x3F8..=0x3FF => {
            // COM1 последовательный порт — эмуляция вывода символа
            if value >= 32 && value < 127 {
                log::info!("OUTSB to COM1 (port 0x3F8): character '{}'", value as char);
            } else {
                log::info!("OUTSB to COM1 (port 0x3F8): byte {:#02x}", value);
            }
            // Для простоты эмуляции можно сохранять вывод в буфер
            if let Some(buffer) = machine.serial_buffer.as_mut() {
                buffer.push(value);
                if value == b'\n' || buffer.len() > 255 {
                    let msg = String::from_utf8_lossy(buffer);
                    log::info!("Serial output: {}", msg);
                    buffer.clear();
                }
            }
        }
        0x378..=0x37A => {
            // LPT1 параллельный порт (принтер)
            log::info!("OUTSB to LPT1 (port 0x378): value {:#02x}", value);
        }
        _ => {
            log::info!("OUTSB to port {:#04x}: value {:#02x} (ignored)", port, value);
        }
    }
    
    // Обновляем указатель в зависимости от флага направления DF (бит 10)
    let df = (machine.registers.flags() & (1 << 10)) != 0;
    if df {
        machine.registers.set_si(si.wrapping_sub(1));
    } else {
        machine.registers.set_si(si.wrapping_add(1));
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn wait(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let bytes = prev.to_vec();
    
    // В эмуляторе без поддержки FPU инструкция является no-op
    // Но логируем для отладки программ, использующих сопроцессор
    log::info!("WAIT/FWAIT executed at {:#04x}:{:#04x} (no-op in emulator without FPU)", 
                machine.registers.cs(), machine.registers.ip());
    
    // Флаги НЕ изменяются — критически важно!
    machine.log_instruction(csip, &bytes).ok();
}