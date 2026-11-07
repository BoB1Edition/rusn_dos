# BETA — Крейты и модули в стадии бета

## Крейты в бета-стадии

### `x86` — Эмуляция CPU 80386+
- Регистры, флаги, ModRM (16/32-bit + SIB)
- Декодер (`execute`/`execute_0f`)
- Все инструкции: ALU, MOV, shift, bit, stack, control, exchange, group, system
- Спецгруппы: `system/` (`control`, `flags`, `interrupt`, `io`, `misc`, `regs`) — SGDT/SIDT/LGDT/LIDT/SMSW/LMSW, MOV CR, SETcc, INT/INTO/IRET, IN/OUT, AAM/AAD/CPUID/RDTSC/WAIT/ARPL
- Трассировка

### `bus` — Шина и железо
- Трейты: `Machine`, `Cpu`, `Display`, `Peripheral`, `Memory`
- `Motherboard` с периферией
- PIC (8259), PIT (8253), клавиатура (8042)
- VGA: CRTC, sequencer, DAC, attribute controller, graphics controller, VBE

### `dos` — DOS-оболочка
- Загрузчики EXE/COM, PSP, EXE-хедер (`loader/`: com_loader, exe_loader, exe_header, psp)
- Файловая система (многодисковая + устройства DOS, `filesystem/` разбит на модули)
- IVT, MCB, EMS (INT 67h), SYSVARS (List of Lists)
- EXEC: `ProcessManager` в `process.rs`, INT 21h AH=4Bh/4Dh
- BAT-интерпретатор: `bat/runner/` (exec, commands, resolve, flow, types, util)
- Прерывания: INT 08h–67h, INT 10h, INT 21h (AH=00h–6Ch), INT 2Fh, INT 33h

### `gui` — Графическое окно
- Окно через minifb
- Клавиатура (scancode → DOS)

### `emu_config` — Конфигурация
- Типы: `Resolution`, `MachineConfig`, `VideoConfig`, `SoundConfig`, `DriverConfig`, `CpuConfig`, `EnvironmentConfig`
- Сериализация через serde + toml

## Примечания
- Бета-код может содержать `todo!()` и `unreachable!()` как заглушки
- Комментарии случайны, не являются документацией
- `#![allow(dead_code)]` допускается
