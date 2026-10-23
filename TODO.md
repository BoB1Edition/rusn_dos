# TODO.md — План развития эмулятора rusn_dos

## Легенда

- `[ ]` — не начато
- `[~]` — в процессе
- `[x]` — готово
- `[!]` — требует исправления/переписывания

## 1. Ядро x86 — Исправление ошибок

- `[x]` IP — сделать 32-битным (`eip`), убрать ограничение `u16`
- `[x]` Регистры — enable `eip` getter/setter, закомментированные в `registers.rs`
- `[!]` `mov_ax_address` (0xA1) / `mov_address_ax` (0xA3) — НЕ dead code, вызываются из executor. Но есть дублирующие `mov_ax_address`/`mov_address_ax` в `mov.rs`, которые не вызываются — надо удалить.
- `[x]` `set_eflags` — до сих пор не форсирует бит 1 в отличие от `set_flags`
- `[!]` AF в shift/rotate — не передаётся (`false`), хотя реальный x86 меняет AF
- `[x]` SGDT/SIDT — до сих пор пишут заглушку
- `[!]` AAA (0x37) и AAS (0x3F) — отсутствуют в `executor.rs`
- `[!]` INT 08h — PIT не эмулирует реальный счётчик, таймер на wall-clock
- `[!]` BSR/BSF (0F BD/BC) — не реализованы
- `[!]` INT1 (0xF1) — заглушка, `halted = true`
- `[!]` WAIT (0x9B) — заглушка (no-op), не эмулирует FSTSW/FNSTSW

## 2. Флаги — Проверка и доработка

- `[x]` DAA/DAS — OF undefined на реальном x86, сохранение старого OF корректно
- `[x]` INC/DEC — AF проверен для всех ширин (u8/u16/u32 через inc_width/dec_width/group_fe)
- `[x]` NEG — AF устанавливается корректно (value & 0x0F != 0) для всех ширин
- `[x]` AAM/AAD — реализованы, SF/ZF/PF по результату, CF/OF/AF сохраняются

## 3. DOS — Доработка API

- `[x]` INT 21h / AH=01h — использует буфер клавиатуры с suspend/resume
- `[x]` INT 21h / AH=06h — DL=0xFF проверяет буфер клавиатуры
- `[x]` INT 10h / AH=0Eh — пишет в видеопамять через write_tty() (текстовый режим)
- `[x]` INT 10h / AH=00h — был сломан копипастой (читал клавиатуру вместо установки режима), исправлено
- `[x]` INT 10h / AH=02h — установка позиции курсора
- `[x]` INT 10h / AH=01h — установка формы курсора
- `[x]` INT 16h / AH=00h — использует suspend_for_key (ожидание клавиши)
- `[x]` FindFirst/FindNext — DTA настраивается через AH=1Ah
- `[x]` INT 21h / AH=1Ah — Set DTA реализован
- `[x]` INT 21h / AH=44h — IOCTL (AL=00h: Get Device Information)
- `[x]` INT 21h / AH=19h — возвращает C: (OK, hardcoded для однодисковой системы)
- `[x]` INT 21h / AH=47h — get current directory — реализован
- `[x]` 8.3 имена в DTA — правильный space-padding через format_83()
- `[x]` stdin/stdout/stderr хендлы 0-4 как device-файлы (CON, AUX, PRN)
- `[x]` INT 21h / AH=0Ah — buffered input (с suspend/resume)
- `[x]` INT 21h / AH=02h, AH=06h, AH=09h, AH=0Ah — вывод через tty_write() в VRAM + stdout
- `[x]` INT 21h / AH=3Fh — чтение из CON (handle=0) через клавиатурный буфер
- `[x]` INT 21h / AH=07h — direct char input, no echo (blocking, no Ctrl+C)
- `[x]` INT 21h / AH=08h — char input, no echo (blocking, with Ctrl+C → INT 23h)
- `[ ]` INT 21h / AH=2Ah — get system date
- `[ ]` INT 21h / AH=2Ch — get system time
- `[ ]` INT 21h / AH=39h — MKDIR
- `[ ]` INT 21h / AH=3Ah — RMDIR
- `[ ]` INT 21h / AH=59h — get extended error
- `[ ]` INT 21h / AH=4B00h — EXEC (загрузка дочерних процессов)
- `[ ]` INT 21h / AH=5Ch — file locking
- `[ ]` INT 21h / AH=71h — long filename (LFN) API

## 4. Периферия — Рефакторинг в плагинную систему

### 4.1. Звук (Plugin: Audio)

- `[ ]` PC Speaker — генерация прямоугольных импульсов, порт 0x61
- `[ ]` Sound Blaster 16 (DSP) — порты 0x220-0x22F, DMA
- `[ ]` OPL2/3 (AdLib/FM синтез) — порты 0x388-0x389
- `[ ]` Микширование и вывод через CPAL/Cpal

### 4.2. Видео (Plugin: Video)

- `[x]` Текстовый режим 80×25 — OK
- `[x]` Mode 13h (320×200×8) — OK
- `[x]` VGA Input Status 1 (порт 0x3DA) — эмуляция retrace тайминга
- `[x]` Attribute Controller flip-flop — сброс при чтении 0x3DA
- `[x]` VGA CRTC — полная эмуляция портов 0x3D4/0x3D5
- `[x]` VGA Attribute Controller — порт 0x3C0/0x3C1, flip-flop, palette mapping
- `[x]` VGA DAC — полная 18-bit палитра (256 цветов)
- `[x]` VGA sequencer — порты 0x3C4/0x3C5
- `[x]` VGA graphics controller — порты 0x3CE/0x3CF
- `[x]` Режимы 12h (640×480×16), 10h (640×350×16)
- `[ ]` CGA/EGA backwards compatibility
- `[ ]` SVGA (VBE) — VESA BIOS Extensions 2.0+
- `[ ]` Оконный GUI-масштабатор с фильтрами (CRT-шаблоны, xBRZ, scale2x)

### 4.3. Ввод (Plugin: Input)

- `[x]` Клавиатура (8042, Set 1 scancodes) — базовая
- `[ ]` Мышь (INT 33h, PS/2)
- `[ ]` Джойстик (Gameport 0x201)
- `[ ]` Поддержка геймпадов через gilrs

### 4.4. Дисковая подсистема (Plugin: Storage)

- `[ ]` INT 13h — BIOS disk services (CHS/LBA)
- `[ ]` Образы дискет (IMG/IMA) — 360KB, 720KB, 1.2MB, 1.44MB
- `[ ]` Образы HDD — VHD, raw images
- `[ ]` CD-ROM — ISO-образы, MSCDEX (INT 2Fh / 15h)
- `[ ]` IDE Controller — порты 0x1F0-0x1F7, PIO/DMA

### 4.5. Сеть (Plugin: Network)

- `[ ]` NE2000 / RTL8029 — эмуляция через TAP/TUN
- `[ ]` Packet Driver API (INT 61h / 66h)
- `[ ]` Winsock / TCP/IP стек (для Win95)
- `[ ]` HTTP-прокси для ретро-браузеров

### 4.6. Таймеры (Plugin: Timers)

- `[!]` PIT (8253) — текущая заглушка на wall-clock при port_read, нужна полноценная счётная эмуляция
- `[ ]` RTC (MC146818) — CMOS RAM, порты 0x70/0x71
- `[ ]` HPET / ACPI — для Windows-гостевых ОС

## 5. Плагинная архитектура

`[ ]` Создать трейт `Plugin` в `bus` с методами `init()`, `tick()`, `port_read/write()`, `mem_read/write()`
`[ ]` Registry плагинов — загрузка из конфига, инициализация, вызов tick()
`[ ]` Опциональная компиляция — feature flags: `audio`, `video`, `storage`, `network`
`[ ]` Отключаемые модули — минимальная сборка без GUI, звука, сети
`[ ]` Hot-reload плагинов (через dlopen) — возможно, но не приоритет

## 6. Другие системы — Заглушки и начало реализации

### 6.1. NES
- `[~]` Заглушка
- `[ ]` Процессор 6502
- `[ ]` PPU (2C02)
- `[ ]` APU (5 каналов)
- `[ ]` Мапперы (MMC1, MMC3, NROM, UNROM)

### 6.2. Sega Mega Drive
- `[~]` Заглушка
- `[ ]` Процессор Motorola 68000
- `[ ]` CPU Z80 (сопроцессор)
- `[ ]` VDP (видео)
- `[ ]` YM2612 (FM + PSG)

### 6.3. SNES
- `[~]` Заглушка
- `[ ]` CPU 65816
- `[ ]` PPU1/PPU2
- `[ ]` SPC700 + DSP

### 6.4. Java Virtual Machine
- `[~]` Заглушка (запуск .class/.jar)
- `[ ]` Интерпретатор байткода (256 опкодов)
- `[ ]` Класс-лоадер из class/jar файлов
- `[ ]` GC (Mark&Sweep или generational)
- `[ ]` Native-интерфейс для вызова API эмулятора

### 6.5. Sega Saturn
- `[~]` Заглушка
- `[ ]` Dual SH-2 SMP
- `[ ]` VDP1 + VDP2
- `[ ]` SCU + DSP

### 6.6. Sega Dreamcast
- `[~]` Заглушка
- `[ ]` SH-4 CPU
- `[ ]` PowerVR2 (CLX2)
- `[ ]` GD-ROM
- `[ ]` AICA звук

## 8. Рефакторинг — Разбивка файлов на модули

- `[x]` `alu.rs` → `alu/` с модулями `core`, `util`, `eval`, `inst`, `bcd`, `incdec`, `group`
- `[x]` `mov.rs` → `mov/` с модулями `rm`, `imm`, `str_ops`
- `[ ]` `shift.rs` — разбить на подкаталог
- `[ ]` `bit.rs` — разбить на подкаталог
- `[ ]` `control.rs` — разбить на подкаталог

## 9. Сборка и CI

- `[ ]` Feature flags: `full`, `minimal`, `x86-only`, `with-audio`, `with-gui`
- `[ ]` Бенчмарки для ключевых инструкций x86
- `[ ]` GitHub Actions — build + test + lint (rustfmt + clippy)
- `[ ]` Fuzz-тестирование декодера ModRM и опкодов
- `[ ]` Интеграционные тесты: запуск реальных DOS .COM/.EXE

## 8. Документация

- `[x]` AGENTS.md — правила для агентов
- `[x]` TODO.md — дорожная карта
- `[x]` docs/architecture.md — описание архитектуры
- `[x]` docs/x86.md — список реализованных инструкций
- `[x]` docs/porting.md — как добавить поддержку новой периферии
- `[x]` docs/bus.md — описание трейтов Machine, Cpu, Peripheral, Display
- `[x]` docs/dos-api.md — полный список поддерживаемых INT 21h функций
- `[x]` docs/build.md — сборка, зависимости, конфигурация
- `[x]` README.md — главный README
- `[x]` DESIGN.md — дизайн-документ
- `[x]` SKILL.md — описание навыков
