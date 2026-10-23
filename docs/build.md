# Build — Сборка, настройка и запуск

## Системные требования

- Rust toolchain (edition 2024): `rustup show` — используйте nightly или stable с поддержкой edition 2024.
- Cargo (поставляется с Rust).

### Зависимости

- **minifb** — для графического окна (зависимость `crate/gui`). На Linux требует `libxkbcommon-dev`, `libwayland-dev` или `libxcb`; на macOS — AppKit (встроен); на Windows — GDI (встроен).
- Остальные зависимости управляются через Cargo.

## Сборка

```bash
# Стандартная сборка
cargo build

# Оптимизированная сборка
cargo build --release

# Только конкретный крейт
cargo build -p x86
cargo build -p dos
cargo build -p bus
```

## Тестирование

```bash
# Все тесты
cargo test

# Только x86 CPU
cargo test -p x86

# Только DOS
cargo test -p dos

# С логами
RUST_LOG=debug cargo test

# Линтер
cargo clippy

# Форматирование
cargo fmt
```

## Запуск DOS-программ

```bash
# Без графики (только stdout)
cargo run Run test/simple.com

# С графическим окном
cargo run Run test/simple.exe --graphics

# С отладочными логами
RUST_LOG=debug cargo run Run test/simple.com

# С трассировкой опкодов (пишет logopcode.txt без вывода в консоль)
cargo run Run test/simple.exe --graphics --no-log

# С указанием рабочей директории
cargo run Run test/prog.exe --workdir /path/to

# С альтернативным конфигом
cargo run Run test/prog.exe --config myconfig.toml
```

## CLI

```
Usage: rusn_dos [OPTIONS] [COMMAND]

Commands:
  Run   Запуск DOS-программы
  Win   Запуск в оконном режиме (TODO)

Options:
  -c, --config <FILE>    Путь к config.toml [default: config.toml]
      --no-log           Отключить трассировку опкодов
      --graphics         Включить графическое окно
```

## Конфигурация (config.toml)

```toml
title = "Biing!"          # Заголовок окна

[resolution]
width = 1024              # Ширина окна
height = 768              # Высота окна

[machine]
a20_enable = false        # A20 gate (false = wrapping on)
ems = false               # EMS 4.0 enable
dpmi = false              # DPMI host enable
#disable_change_flags = ["a20_enable", "ems"]  # Блокировка изменения флагов из программ

[video]
type = "vga"
available_types = ["vga", "svga"]

[sound]
type = "pc_speaker"
available_types = ["pc_speaker", "sb16"]

[[drivers]]
letter = "C"              # Буква диска
path = "./app"            # Путь к каталогу на хосте
read_only = false         # Только чтение

[[drivers]]
letter = "Z"
path = "/tmp/mgk/"
read_only = true
```

### Драйверы дисков

Каждый `[[drivers]]` маппит букву DOS-диска на каталог на хосте:

- `letter` — буква диска (A-Z, регистронезависимая).
- `path` — путь к каталогу (относительный или абсолютный).
- `read_only` — если `true`, файлы не создаются и не изменяются.

Диск C: по умолчанию должен быть определён (многие программы ожидают его наличие).

## Структура проекта

```
├── crate/
│   ├── bus/          # Трейты + Motherboard + PIC/PIT/VGA/Keyboard
│   ├── x86/          # 80386+ CPU (регистры, флаги, все инструкции)
│   ├── dos/          # DOS-оболочка (INT 21h, BIOS, EMS, MCB, FS)
│   ├── gui/          # Графическое окно (minifb)
│   ├── emu_config/   # Типы конфигурации (Resolution, MachineConfig и др.)
│   ├── nes/          # NES (заглушка)
│   ├── megadrive/    # Sega Mega Drive (заглушка)
│   ├── snes/         # SNES (заглушка)
│   ├── jvm/          # Java VM (заглушка)
│   ├── saturn/       # Sega Saturn (заглушка)
│   └── dreamcast/    # Sega Dreamcast (заглушка)
├── src/              # Точка входа (clap CLI)
├── docs/             # Документация
├── test/             # Тестовые DOS-программы
└── config.toml       # Пример конфигурации
```

## Профили сборки

```toml
# .cargo/config.toml (рекомендуемые настройки для разработки)
[profile.dev]
opt-level = 1          # Ускоряет тесты, не замедляя компиляцию
debug = true

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

## Переменные окружения

- `RUST_LOG=info|debug|trace` — уровень логирования.
- `RUST_BACKTRACE=1` — печать стека при панике.
