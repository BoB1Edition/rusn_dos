# AGENTS.md — Правила и соглашения для агентов, работающих с кодом

## Проект: rusn_dos — Мультисистемный эмулятор на Rust

### Архитектура

- `crate/` — workspace с крейтами: `x86`, `bus`, `dos`, `gui`, `nes`, `megadrive`, `snes`, `jvm`, `saturn`, `dreamcast`
- `crate/bus` — шина, трейты `Machine`, `Cpu`, `Display`, `Peripheral`, `Memory`, примитивы
- `crate/x86` — эмуляция CPU 80386+: регистры, флаги, ModRM, декодер, все инструкции
- `crate/dos` — DOS-оболочка: загрузчики EXE/COM, файловая система, IVT, MCB, EMS, прерывания (INT 08h–67h)
- `crate/gui` — графическое окно (minifb), клавиатура
- `src/` — точка входа, конфигурация, монтирование дисков

### Стиль кода

- rustfmt с настройками по умолчанию (edition 2024)
- `clippy` — все предупреждения должны быть устранены
- Комментарии только где необходимо объяснить нетривиальную логику
- Все ошибки обрабатывать через `Result<T, Box<dyn Error>>` или логи
- `todo!()` и `unreachable!()` допустимы только как заглушки на время разработки
- Вектора с capacity: `Vec::with_capacity(n)` при известном размере

### Разработка инструкций x86

- Каждая группа инструкций в отдельном файле `crate/x86/src/instructions/`
- ModRM разбирается через `ModRm::from_byte()` + `resolve_address()`
- Префиксы 0x66/0x67 обрабатываются через `cpu.prefixes.has_operand_size/has_address_size`
- Флаги: `compute_flags_u8/16/32()` для арифметики, `compute_logical_flags_*()` для логики
- INC/DEC не меняют CF — сохранять через `old_cf = flags & CF`
- 0x0F двухбайтовые опкоды — через `execute_0f()`

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
```

### Соглашения

- DOS API (INT 21h) — все функции в `crate/dos/src/interrupts/dos.rs`
- BIOS API (INT 10h/16h/15h/1Ah) — в `crate/dos/src/interrupts/bios.rs`
- EMS 4.0 (INT 67h) — в `crate/dos/src/interrupts/ems.rs`
- IVT инициализируется в `crate/dos/src/ivt.rs`
- Железо: PIC, PIT, клавиатура, VGA — в `crate/bus/src/peripherals/`
- Новый крейт добавлять в `[workspace] members` в корневом `Cargo.toml`

### Приоритетные направления

1. x86 real mode — корректность опкодов и флагов
2. Загрузчики EXE/COM — полная поддержка MZ и COM
3. DOS API — INT 21h/AH=00h-6Ch, INT 2Fh
4. VGA — режимы 03h, 13h, палитра, курсор
5. Звук — заглушки под PC Speaker, SB16
6. Плагинная архитектура для отключаемых модулей
