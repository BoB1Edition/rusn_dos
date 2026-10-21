# rusn_dos — Мультисистемный эмулятор на Rust

rusn_dos — это мультисистемный эмулятор, написанный на Rust (edition 2024). Основная цель — эмуляция IBM PC-совместимых DOS-систем с полным процессором 80386+ и окружением DOS.

## Возможности

- **CPU**: 80386+ real mode, полный набор инструкций (300+ опкодов), ModRM с SIB, префиксы 0x66/0x67/REP/seg, флаги
- **DOS**: Загрузка COM/EXE (MZ), INT 21h (AH=00h-6Ch), INT 2Fh, файловая система с несколькими дисками, устройства DOS (CON/NUL/COM/LPT/CLOCK$/EMMXXXX0)
- **BIOS**: INT 10h (видеорежимы, телетайп), INT 16h (клавиатура), INT 15h (A20, расширенная память), INT 1Ah (часы), INT 08h/1Ch (таймер)
- **EMS 4.0**: INT 67h (выделение/освобождение/маппинг страниц)
- **VGA**: Текстовый режим 80x25, Mode 13h (320x200x8), рендеринг через minifb
- **Клавиатура**: 8042, Set 1 scancodes, буфер BIOS, suspend/resume для неблокирующего ввода
- **MCB**: DOS Memory Control Block — выделение/освобождение/изменение блоков памяти
- **Графическое окно**: minifb, 60 FPS, маппинг клавиш

## Быстрый старт

```bash
cargo build
cargo run Run test/simple.com
cargo run Run test/simple.exe --graphics
RUST_LOG=debug cargo run Run test/simple.com
```

## Структура проекта

```
├── crate/
│   ├── bus/          # Трейты (Machine, Cpu, Display, Peripheral) + Motherboard + PIC/PIT/VGA/Keyboard
│   ├── x86/          # 80386+ CPU (регистры, флаги, ModRM, декодер, все инструкции)
│   ├── dos/          # DOS-слой (загрузчики, INT 21h/10h/16h/67h, EMS, MCB, файловая система)
│   ├── gui/          # Окно minifb, клавиатура
│   ├── emu_config/   # Типы конфигурации
│   ├── nes/          # (заглушка)
│   ├── megadrive/    # (заглушка)
│   ├── snes/         # (заглушка)
│   ├── jvm/          # (заглушка)
│   ├── saturn/       # (заглушка)
│   └── dreamcast/    # (заглушка)
├── src/              # Точка входа (clap CLI), конфигурация, монтирование дисков
├── docs/             # Документация
├── test/             # Тестовые DOS-программы
└── config.toml       # Конфигурация эмулятора
```

## Конфигурация

Эмулятор настраивается через `config.toml`:

```toml
title = "Biing!"

[resolution]
width = 1024
height = 768

[machine]
a20_enable = false
ems = false
dpmi = false

[video]
type = "vga"
available_types = ["vga", "svga"]

[sound]
type = "pc_speaker"
available_types = ["pc_speaker", "sb16"]

[[drivers]]
letter = "C"
path = "./app"
read_only = false
```

## CLI

```bash
rusn_dos Run <program> [--workdir <dir>] [--graphics] [--no-log] [--config <path>]
rusn_dos Win [--workdir <dir>] [--config <path>]
```

## Документация

- [Архитектура](docs/architecture.md)
- [Список инструкций x86](docs/x86.md)
- [Портирование](docs/porting.md)
- [План развития](TODO.md)
- [Известные ошибки](ERRORS.md)
- [Приоритетные опкоды](topopcodes.md)

## Тестирование

```bash
cargo test                  # Все тесты
cargo test -p x86           # Только x86 CPU
cargo test -p dos           # Только DOS
RUST_LOG=debug cargo test   # С логами
```

## Лицензия

MIT
