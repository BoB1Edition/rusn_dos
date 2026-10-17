# TODO.md — План развития эмулятора rusn_dos

## Легенда

- `[ ]` — не начато
- `[~]` — в процессе
- `[x]` — готово
- `[!]` — требует исправления/переписывания

## 1. Ядро x86 — Исправление ошибок

- `[!]` IP — сделать 32-битным (`eip`), убрать ограничение `u16`
- `[!]` Регистры — enable `eip` getter/setter, закомментированные в `registers.rs`
- `[!]` `mov_ax_address` / `mov_address_ax` — dead code, не вызываются из executor
- `[!]` `set_eflags` не форсирует бит 1 в отличие от `set_flags`
- `[!]` Парность (PF) — проверить на u8 low-byte для всех ширин
- `[!]` AF в shift/rotate — не передаётся (`false`), хотя реальный x86 меняет AF
- `[!]` SGDT/SIDT — пишут заглушку (0xFFFF, 0), а не реальные GDTR/IDTR
- `[!]` AAA (0x37) и AAS (0x3F) — отсутствуют в `executor.rs`
- `[!]` INT 08h — PIT не эмулирует реальный счётчик, таймер на wall-clock
- `[!]` BSR/BSF (0F BD/BC) — не реализованы
- `[!]` INT1 (0xF1) — заглушка, `halted = true`
- `[~]` WAIT (0x9B) — заглушка (no-op), не эмулирует FSTSW/FNSTSW

## 2. Флаги — Проверка и доработка

- `[!]` DAA/DAS — OF не меняется на реальном CPU (undefined), но читается
- `[!]` INC/DEC — CF не меняется (OK), проверить AF для всех ширин
- `[!]` NEG — проверить AF (должен устанавливаться если исходное значение & 0x0F != 0)
- `[!]` AAM/AAD — сохраняют CF/OF/AF от старого состояния (реальный x86: SF/ZF/PF по результату, AF/CF/OF undefined)

## 3. DOS — Доработка API

- `[~]` INT 21h / AH=01h — чтение из stdin блокирует эмулятор (исправить на буфер клавиатуры)
- `[!]` INT 21h / AH=06h — DL=0xFF всегда возвращает "нет клавиши"
- `[!]` INT 10h / AH=0Eh — не пишет в видеопамять в режиме 13h
- `[!]` INT 10h / AH=02h — установка позиции курсора — заглушка
- `[!]` INT 16h / AH=00h — должен ждать клавишу, а не возвращать 0
- `[!]` FindFirst/FindNext — DTA всегда DS:0x0080, нет поддержки AH=1Ah (set DTA)
- `[!]` INT 21h / AH=19h — всегда возвращает C: (должен быть текущий)
- `[!]` 8.3 имена в DTA — нет proper space-padding, что ломает некоторые программы
- `[ ]` INT 21h / AH=0Ah — buffered input
- `[ ]` INT 21h / AH=2Ah — get system date
- `[ ]` INT 21h / AH=2Ch — get system time
- `[ ]` INT 21h / AH=39h — MKDIR
- `[ ]` INT 21h / AH=3Ah — RMDIR
- `[ ]` INT 21h / AH=44h — IOCTL (ioctl для устройств)
- `[ ]` INT 21h / AH=59h — get extended error
- `[ ]` INT 21h / AH=4B00h — EXEC (загрузка дочерних процессов)
- `[ ]` INT 21h / AH=5Ch — file locking
- `[ ]` INT 21h / AH=71h — long filename (LFN) API
- `[ ]` stdin/stdout/stderr хендлы 0-4 как настоящие файловые дескрипторы

## 4. Периферия — Рефакторинг в плагинную систему

### 4.1. Звук (Plugin: Audio)

- `[ ]` PC Speaker — генерация прямоугольных импульсов, порт 0x61
- `[ ]` Sound Blaster 16 (DSP) — порты 0x220-0x22F, DMA
- `[ ]` OPL2/3 (AdLib/FM синтез) — порты 0x388-0x389
- `[ ]` Микширование и вывод через CPAL/Cpal

### 4.2. Видео (Plugin: Video)

- `[~]` Текстовый режим 80×25 — OK
- `[~]` Mode 13h (320×200×8) — OK
- `[ ]` VGA CRTC — полная эмуляция портов 0x3D4/0x3D5
- `[ ]` VGA DAC — полная 18-bit палитра (256 цветов)
- `[ ]` VGA sequencer — порты 0x3C4/0x3C5
- `[ ]` VGA graphics controller — порты 0x3CE/0x3CF
- `[ ]` Режимы 12h (640×480×16), 10h (640×350×16)
- `[ ]` CGA/EGA backwards compatibility
- `[ ]` SVGA (VBE) — VESA BIOS Extensions 2.0+
- `[ ]` Оконный GUI-масштабатор с фильтрами (CRT-шаблоны, xBRZ, scale2x)

### 4.3. Ввод (Plugin: Input)

- `[~]` Клавиатура (8042, Set 1 scancodes) — базовая
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

- `[!]` PIT (8253) — текущая заглушка на wall-clock, нужна полноценная счётная эмуляция
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

## 7. Сборка и CI

- `[ ]` Feature flags: `full`, `minimal`, `x86-only`, `with-audio`, `with-gui`
- `[ ]` Бенчмарки для ключевых инструкций x86
- `[ ]` GitHub Actions — build + test + lint (rustfmt + clippy)
- `[ ]` Fuzz-тестирование декодера ModRM и опкодов
- `[ ]` Интеграционные тесты: запуск реальных DOS .COM/.EXE

## 8. Документация

- `[~]` AGENTS.md — правила для агентов
- `[~]` TODO.md — дорожная карта
- `[ ]` docs/architecture.md — описание архитектуры
- `[ ]` docs/x86.md — список реализованных инструкций
- `[ ]` docs/porting.md — как добавить поддержку новой периферии
