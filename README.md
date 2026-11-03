# rusn_dos — Мультисистемный эмулятор на Rust

rusn_dos — это мультисистемный эмулятор, написанный на Rust (edition 2024). Основная цель — эмуляция IBM PC-совместимых DOS-систем с полным процессором 80386+ и окружением DOS.

## Возможности

- **CPU**: 80386+ real mode, полный набор инструкций (300+ опкодов), ModRM с SIB, префиксы 0x66/0x67/REP/seg, флаги
- **DOS**: Загрузка COM/EXE (MZ), INT 21h (AH=00h-6Ch), INT 2Fh, INT 33h (break check, boot drive, версия DOS), файловая система с несколькими дисками, устройства DOS (CON/NUL/COM/LPT/CLOCK$/EMMXXXX0), поиск файлов и программ по PATH
- **BAT-интерпретатор**: `.BAT`/`.CMD` уровня COMMAND.COM — echo, cd/cls/goto (вперёд и назад), `if errorlevel N`, `if exist`, set, pause, call, exit, rem, смена диска, раскрытие `%1`–`%9` и `%VAR%`, глубина вложенности через `--max-call-depth`
- **EXEC**: INT 21h/AH=4Bh/AL=00 (запуск дочерних процессов из DOS-программ) + AH=4Dh (код возврата), снимок/восстановление родителя
- **BIOS**: INT 10h (AH=00h–0Fh, 12h: видеорежимы, курсор, скроллинг, вывод символов, альтернативные функции), INT 16h (клавиатура), INT 15h (A20, расширенная память), INT 1Ah (часы), INT 08h/1Ch (таймер)
- **EMS 4.0**: INT 67h (выделение/освобождение/маппинг страниц)
- **VGA**: Текстовый режим 80x25, Mode 13h (320x200x8), планарные режимы 0Dh/0Eh/10h/12h (16 цветов), DAC 18-bit палитра (256 цветов), VGA sequencer + graphics controller + CRTC + attribute controller, VBE (VESA BIOS Extensions: 640x480x256 / 800x600x256 / 1024x768x256 / 24-bit), порты 0x3C0-0x3DA, Input Status 1 (retrace timing), attribute flip-flop, рендеринг через minifb
- **Клавиатура**: 8042, Set 1 scancodes, буфер BIOS, prev_keys (только новые нажатия), suspend/resume (re-dispatch) для неблокирующего ввода
- **Окружение DOS**: переменные PATH/COMSPEC/PROMPT из `[environment]` в config.toml, блок окружения строится в PSP при загрузке
- **MCB**: DOS Memory Control Block — выделение/освобождение/изменение блоков памяти
- **Графическое окно**: minifb, 60 FPS, маппинг клавиш
- **Stdin Bridge**: crossterm raw-mode опрос клавиатуры через stdin в консольном режиме, маппинг US scancodes

## Быстрый старт

```bash
cargo build
cargo run Run test/test1
cargo run Run test/test2 --graphics
cargo run Run test/build.bat --graphics      # TASM-сборка внутри DOS
cargo run Run app/XCOM/GO.COM --graphics -w C:\\XCOM
RUST_LOG=debug cargo run Run test/test1
```

## Структура проекта

```
├── crate/
│   ├── bus/          # Трейты (Machine, Cpu, Display, Peripheral) + Motherboard + PIC/PIT/VGA/Keyboard
│   │   └── peripherals/video/  # VGA — core, dac, planar, render, peripheral, vbe, types
│   ├── x86/          # 80386+ CPU (регистры, флаги, ModRM, декодер)
│   │   └── instructions/  # alu/ (core, util, eval, inst, bcd, incdec, group), mov/ (rm, imm, str_ops), bit, shift, control, exchange, stack, group, system
│   ├── dos/          # DOS-слой (загрузчики, BAT-интерпретатор, INT 21h/10h/16h/67h, EMS, MCB, файловая система)
│   │   ├── bat/          # runner.rs — BAT-интерпретатор уровня COMMAND.COM
│   │   └── interrupts/dos/  # dispatch, console, file_ops, dir, find, memory, exec, misc
│   ├── emu_config/   # Типы конфигурации (включая EnvironmentConfig)
│   ├── nes/          # (заглушка)
│   ├── megadrive/    # (заглушка)
│   ├── snes/         # (заглушка)
│   ├── jvm/          # (заглушка)
│   ├── saturn/       # (заглушка)
│   └── dreamcast/    # (заглушка)
├── src/              # Точка входа (clap CLI), конфигурация, монтирование дисков, stdin_bridge, маршрутизация .BAT
├── docs/             # Документация
├── test/             # Тестовые DOS-программы (.asm) и .BAT
├── app/              # Монтируется как диск C: (примеры: XCOM, biing, TASM, test)
├── test.sh           # Скрипт прогона целевых программ
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

[environment]
PATH = "C:\\;C:\\TASM\\BIN"
COMSPEC = "C:\\COMMAND.COM"
PROMPT = "$P$G"

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
  -s, --startdir <DIR>       Поиск программы [default: ./]
  -w, --workdir <DIR>        DOS-рабочая директория (например -w C:\\XCOM) [default: ./]
      --graphics             Включить графическое окно
      --no-stdin-keys        Не читать stdin как клавиши (headless)
      --mount <L=path[:ro]>  Монтирование диска (объявлено, в работе)
      --max-call-depth <N>   Лимит глубины CALL в BAT-файлах (1..=255) [default: 20]
```

Коды выхода:
- `0` — успешное завершение программы (код выхода DOS-программы)
- `1` — ошибка эмулятора (CLI, конфиг, путей, валидации `--max-call-depth`)
- `101` — паника Rust

## Документация

- [Архитектура](docs/architecture.md)
- [Список инструкций x86](docs/x86.md)
- [Трейты и шина](docs/bus.md)
- [DOS API reference](docs/dos-api.md)
- [Сборка и настройка](docs/build.md)
- [Портирование](docs/porting.md)
- [Цели и приоритеты](GOALS.md)
- [План развития](TODO.md)
- [Известные ошибки](ERRORS.md)
- [Дизайн](DESIGN.md)
- [Памятка для агентов](AGENTS.md)

## Тестирование

```bash
cargo test -p x86           # Тесты CPU
./test.sh                   # Прогон целевых программ (build + run)
cargo run Run test/chain.bat --graphics hello
cargo run Run test/fail.bat --graphics
cargo run Run test/recurce.bat --graphics -d 300   # проверка лимита глубины
```

## Лицензия

MIT
