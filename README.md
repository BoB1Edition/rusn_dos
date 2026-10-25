# rusn_dos — Мультисистемный эмулятор на Rust

rusn_dos — это мультисистемный эмулятор, написанный на Rust (edition 2024). Основная цель — эмуляция IBM PC-совместимых DOS-систем с полным процессором 80386+ и окружением DOS.

## Возможности

- **CPU**: 80386+ real mode, полный набор инструкций (300+ опкодов), ModRM с SIB, префиксы 0x66/0x67/REP/seg, флаги
- **DOS**: Загрузка COM/EXE (MZ), INT 21h (AH=00h-6Ch), INT 2Fh, файловая система с несколькими дисками, устройства DOS (CON/NUL/COM/LPT/CLOCK$/EMMXXXX0)
- **BIOS**: INT 10h (AH=00h–0Fh, 12h: видеорежимы, курсор, скроллинг, вывод символов, альтернативные функции), INT 16h (клавиатура), INT 15h (A20, расширенная память), INT 1Ah (часы), INT 08h/1Ch (таймер)
- **EMS 4.0**: INT 67h (выделение/освобождение/маппинг страниц)
- **VGA**: Текстовый режим 80x25, Mode 13h (320x200x8), планарные режимы 0Dh/0Eh/10h/12h (16 цветов), DAC 18-bit палитра (256 цветов), VGA sequencer + graphics controller + CRTC + attribute controller, VBE (VESA BIOS Extensions: 640x480x256 / 800x600x256 / 1024x768x256 / 24-bit), порты 0x3C0-0x3DA, Input Status 1 (retrace timing), attribute flip-flop, рендеринг через minifb
- **Клавиатура**: 8042, Set 1 scancodes, буфер BIOS, prev_keys (только новые нажатия), suspend/resume (re-dispatch) для неблокирующего ввода
- **MCB**: DOS Memory Control Block — выделение/освобождение/изменение блоков памяти
- **Графическое окно**: minifb, 60 FPS, маппинг клавиш
- **Stdin Bridge**: crossterm raw-mode опрос клавиатуры через stdin в консольном режиме, маппинг US scancodes

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
│   │   └── peripherals/video/  # VGA — core, dac, planar, render, peripheral, vbe, types
│   ├── x86/          # 80386+ CPU (регистры, флаги, ModRM, декодер)
│   │   └── instructions/  # alu/ (core, util, eval, inst, bcd, incdec, group), mov/ (rm, imm, str_ops), bit, shift, control, exchange, stack, group, system
│   ├── dos/          # DOS-слой (загрузчики, INT 21h/10h/16h/67h, EMS, MCB, файловая система)
│   │   └── interrupts/dos/  # dispatch, console, file_ops, dir, find, memory, misc
│   ├── emu_config/   # Типы конфигурации
│   ├── nes/          # (заглушка)
│   ├── megadrive/    # (заглушка)
│   ├── snes/         # (заглушка)
│   ├── jvm/          # (заглушка)
│   ├── saturn/       # (заглушка)
│   └── dreamcast/    # (заглушка)
├── src/              # Точка входа (clap CLI), конфигурация, монтирование дисков, stdin_bridge
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
rusn_dos [OPTIONS] <SUBCOMMAND>

SUBCOMMANDS:
  Run <PROGRAM> [--startdir <DIR>] [--workdir <DIR>] [--] [<args>...]
  Win [--workdir <DIR>]

OPTIONS:
  -c, --config <FILE>        Путь к config.toml [default: config.toml]
      --no-log               Отключить трассировку опкодов
      --graphics             Включить графическое окно
      --no-stdin-keys        Не читать stdin как клавиши
      --mount <L=path[:ro]>  Монтирование диска
      --max-call-depth <N>   Лимит глубины CALL [default: 20]
```

## Документация

- [Архитектура](docs/architecture.md)
- [Список инструкций x86](docs/x86.md)
- [Трейты и шина](docs/bus.md)
- [DOS API reference](docs/dos-api.md)
- [Сборка и настройка](docs/build.md)
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
