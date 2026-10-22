# AGENTS.md — Правила и соглашения для агентов, работающих с кодом

## Проект: rusn_dos — Мультисистемный эмулятор на Rust

### Архитектура

- `crate/` — workspace с крейтами: `x86`, `bus`, `dos`, `gui`, `emu_config`, `nes`, `megadrive`, `snes`, `jvm`, `saturn`, `dreamcast`
- `crate/bus` — шина, трейты `Machine`, `Cpu`, `Display`, `Peripheral`, `Memory`, примитивы; материнская плата `Motherboard` с периферией (PIC, PIT, клавиатура, VGA)
- `crate/x86` — эмуляция CPU 80386+: регистры, флаги, ModRM (16/32-bit + SIB), декодер (`execute`/`execute_0f`), все инструкции
- `crate/dos` — DOS-оболочка: загрузчики EXE/COM, файловая система (многодисковая + устройства DOS), IVT, MCB, EMS, прерывания (INT 08h–67h)
- `crate/gui` — графическое окно (minifb), клавиатура (scancode → DOS)
- `crate/emu_config` — типы конфигурации (Resolution, MachineConfig, VideoConfig, SoundConfig, DriverConfig)
- `src/` — точка входа (clap CLI), конфигурация (config.toml), монтирование дисков
- максимальное избегание длинных более 300 строк файлов исходников, длинные файлы допустимы только на время беты, группировка по логике в отдельный каталогах
- максимальное избегание god object где только можно
- использование макросов для избегания дублирования кода приветствуется
- переход на альфу предполагает перевод кахдого процессора\сопроцессора на свой поток, то есть видео карта - поток, аудио - отдельный поток, таймер отдельный поток....  

### Стиль кода

- rustfmt с настройками по умолчанию (edition 2024)
- `clippy` — все предупреждения должны быть устранены
- Комментарии случайны
- Все ошибки обрабатывать через `Result<T, Box<dyn Error>>` или логи
- `todo!()` и `unreachable!()` допустимы только как заглушки на время разработки
- Вектора с capacity: `Vec::with_capacity(n)` при известном размере


### Разработка инструкций x86

- Каждая группа инструкций в отдельном файле `crate/x86/src/instructions/`
- ModRM разбирается через `ModRm::from_byte()` + `resolve_address()` / `resolve_offset()`
- Префиксы 0x66/0x67 обрабатываются через `cpu.prefixes.has_operand_size / has_address_size`
- Флаги: `compute_flags_u8/16/32()` для арифметики, `compute_logical_flags_*()` для логики
- INC/DEC не меняют CF — сохранять через `old_cf = flags & CF`
- 0x0F двухбайтовые опкоды — через `execute_0f()`
- `fetch_u8/u16/u32` автоматически инкрементируют EIP и пишут в `instruction_bytes` для трассировки

### Разработка для других систем

- Каждая система — отдельный крейт в `crate/`
- Должна реализовать базовый трейт `Cpu` или предоставить `step()`
- `load_rom()` / `load_cd_image()` / `load_class()` — загрузка контента
- `DosSystem` — основная интеграционная шина (x86 + DOS + будущие системы)

### Сборка и тестирование

```bash
cargo build
cargo run Run test/simple.com
cargo run Run test/simple.exe --graphics
cargo test -p x86       # модульные тесты CPU
RUST_LOG=debug cargo run Run test/simple.com
```

### Соглашения

- EMS 4.0 (INT 67h) — в `crate/dos/src/interrupts/ems.rs`
- IVT инициализируется в `crate/dos/src/ivt.rs` (вектора с CS=0xF000 — внутренние)
- Железо: PIC (8259), PIT (8253), клавиатура (8042), VGA — в `crate/bus/src/peripherals/`
- Устройства DOS: `tty_write()` — пишет и в VRAM и в stdout (используется INT 21h/AH=02h,06h,09h,0Ah)
- Механизм suspend/resume для неблокирующего ввода: `SuspendedCall` в `system.rs` (re-dispatch через `dispatch(vector)`, не stack-unwind)

### Приоритетные направления

1. x86 real mode — корректность опкодов и флагов
2. Загрузчики EXE/COM — полная поддержка MZ и COM
3. DOS API — INT 21h/AH=00h-6Ch, INT 2Fh (включая AH=0Ah, 4B00h)
4. VGA — планарные режимы (0Dh/0Eh/10h/12h), DAC 18-bit палитра, sequencer/graphics controller, INT 10h (AH=00h–0Fh, 12h), курсор
5. Звук — реализация PC Speaker, SB, SB16, SB pro, UltraSound, Pro Audio Spectrum, Pro Audio Spectrum 16
6. Поддержка nes
6. Поддержка sega megadrive
