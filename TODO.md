# TODO.md — План развития эмулятора rusn_dos

## Легенда

- `[ ]` — не начато
- `[~]` — в процессе
- `[x]` — готово
- `[!]` — требует исправления/переписывания

## 1. Ядро x86 — Исправление ошибок

- `[x]` IP — сделать 32-битным (`eip`), убрать ограничение `u16`
- `[x]` Регистры — enable `eip` getter/setter, закомментированные в `registers.rs`
- `[!]` `mov_ax_address` (0xA1) / `mov_address_ax` (0xA3) — НЕ dead code, реализованы в `mov/imm.rs`, вызываются из executor. Устаревший пункт — проверено 03.11.2026.
- `[x]` `set_eflags` — до сих пор не форсирует бит 1 в отличие от `set_flags` → **не исправлено** (см. ERRORS.md)
- `[!]` AF в shift/rotate — не передаётся (`false`), хотя реальный x86 меняет AF
- `[x]` SGDT/SIDT — реализованы (пишут limit+base в память) в `system/control.rs`
- `[!]` AAA (0x37) и AAS (0x3F) — отсутствуют в `executor.rs`
- `[!]` INT 08h — PIT не эмулирует реальный счётчик, таймер на wall-clock
- `[!]` BSR/BSF (0F BD/BC) — не реализованы
- `[!]` INT1 (0xF1) — заглушка (`halted = true`)
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
- `[x]` INT 21h / AH=2Ah — get system date
- `[x]` INT 21h / AH=2Ch — get system time
- `[x]` INT 21h / AH=38h — get country info (GET/SET, текущий code = 1/USA)
- `[x]` INT 21h / AH=33h — break check (get/set/swap), boot drive, версия DOS 6.22
- `[x]` INT 21h / AH=4B00h — EXEC (загрузка дочерних процессов через `ProcessManager::start_child()`/`finish_child()`)
- `[x]` INT 21h / AH=4Dh — получить код возврата ребёнка
- `[x]` INT 21h / AH=50h/51h/62h — Set/Get PSP (два геттера — 51h и 62h)
- `[x]` INT 21h / AH=52h — Get List of Lists (SYSVARS, инициализируется `sys.init_sysvars()` в loader/mod.rs)
- `[x]` INT 21h / AH=57h — get/set file date/time (get: реальные timestamps; set: no-op с CF=0)
- `[ ]` INT 21h / AH=39h — MKDIR
- `[ ]` INT 21h / AH=3Ah — RMDIR
- `[ ]` INT 21h / AH=59h — get extended error
- `[ ]` INT 21h / AH=4B01h — EXEC load-only (возвращает CF=1/AX=1)
- `[ ]` INT 21h / AH=4B03h — EXEC overlay (возвращает CF=1/AX=1)
- `[ ]` INT 21h / AH=5Ch — file locking
- `[ ]` INT 21h / AH=71h — long filename (LFN) API

## 4. Периферия — Рефакторинг в плагинную систему

### 4.1. Звук (Plugin: Audio)

- `[ ]` PC Speaker — генерация прямоугольных импульсов, порт 0x61
- `[ ]` Sound Blaster 16 (DSP) — порты 0x220-0x22F, DMA
- `[ ]` OPL2/3 (AdLib/FM синтез) — порты 0x388-0x389
- `[ ]` Микширование и вывод через CPAL/CPAL

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
- `[x]` SVGA (VBE) — VESA BIOS Extensions: режимы 101h, 103h, 105h, 112h; LFB, bank switching
- `[ ]` CGA/EGA backwards compatibility
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

- `[ ]` Создать трейт `Plugin` в `bus` с методами `init()`, `tick()`, `port_read/write()`, `mem_read/write()`
- `[ ]` Registry плагинов — загрузка из конфига, инициализация, вызов tick()
- `[ ]` Опциональная компиляция — feature flags: `audio`, `video`, `storage`, `network`
- `[ ]` Отключаемые модули — минимальная сборка без GUI, звука, сети
- `[ ]` Hot-reload плагинов (через dlopen) — возможно, но не приоритет

## 6. BAT-интерпретатор (COMMAND.COM уровень)

- `[x]` Модуль `crate/dos/src/bat/runner/` — подкаталог с модулями `exec.rs`, `commands.rs`, `resolve.rs`, `flow.rs`, `types.rs`, `util.rs` (вариант B: каждый ребёнок в свежем `DosSystem`)
- `[x]` `ShellState` — CWD по дискам, текущий диск, env, echo, errorlevel
- `[x]` Встроенные: echo, cd/chdir, cls, goto, if errorlevel N, if exist, set, pause, call, exit, rem, смена диска
- `[x]` Раскрытие `%0`–`%9`, `%VAR%`, `%%`→`%`, `@`-подавление эха
- `[x]` `--max-call-depth` (1..=255, default 20), глубина `if` ≤ 4
- `[x]` `resolve_command()` — поиск COM/EXE/BAT: cwd → PATH → программа
- `[x]` `pause` в консоли (pump) и в графике (poll_keys окна); `PumpFn` для неблокирующего pump'а из BAT
- `[ ]` CALL :label (подпрограммы) — на Этапе 4 не поддерживается (печатает сообщение)
- `[ ]` FOR, SHIFT, ERRORLEVEL как переменная, `if defined VAR`
- `[ ]` `set /a`, `set /p` (арифметика/ввод)
- `[ ]` `choice`, `fsubst`, `path`, `prompt` встроенные

## 7. Другие системы — Заглушки и начало реализации

### 7.1. NES
- `[~]` Заглушка
- `[ ]` Процессор 6502
- `[ ]` PPU (2C02)
- `[ ]` APU (5 каналов)
- `[ ]` Мапперы (MMC1, MMC3, NROM, UNROM)

### 7.2. Sega Mega Drive
- `[~]` Заглушка
- `[ ]` Процессор Motorola 68000
- `[ ]` CPU Z80 (сопроцессор)
- `[ ]` VDP (видео)
- `[ ]` YM2612 (FM + PSG)

### 7.3. SNES
- `[~]` Заглушка
- `[ ]` CPU 65816
- `[ ]` PPU1/PPU2
- `[ ]` SPC700 + DSP

### 7.4. Java Virtual Machine
- `[~]` Заглушка (запуск .class/.jar)
- `[ ]` Интерпретатор байткода (256 опкодов)
- `[ ]` Класс-лоадер из class/jar файлов
- `[ ]` GC (Mark&Sweep или generational)
- `[ ]` Native-интерфейс для вызова API эмулятора

### 7.5. Sega Saturn
- `[~]` Заглушка
- `[ ]` Dual SH-2 SMP
- `[ ]` VDP1 + VDP2
- `[ ]` SCU + DSP

### 7.6. Sega Dreamcast
- `[~]` Заглушка
- `[ ]` SH-4 CPU
- `[ ]` PowerVR2 (CLX2)
- `[ ]` GD-ROM
- `[ ]` AICA звук

## 8. Консольный ввод (Stdin Bridge)

- `[x]` `src/stdin_bridge.rs` — crossterm raw-mode опрос stdin
- `[x]` US scancode маппинг (буквы, цифры, F1-F10, стрелки, home/end/pgup/pgdn, ins/del)
- `[x]` `RawTerminal` RAII-гард — try_enable() для tty, None для пайпов
- `[x]` `poll_keys()` — неблокирующий опрос всех накопленных событий
- `[x]` `--no-stdin-keys` отключает stdin bridge
- `[x]` При `suspended` + `--no-stdin-keys` → ошибка
- `[x]` `NO_STDIN_KEYS_ERR` — единая константа сообщения

## 9. Окружение и поиск программ

- `[x]` `EnvironmentConfig` в `emu_config` (переменные, `get()`, `path_entries()`)
- `[x]` Секция `[environment]` в config.toml (PATH/COMSPEC/PROMPT)
- `[x]` `build_environment_block()` строит блок из переменных + динамический сегмент
- `[x]` `App::resolve_program_path()` — поиск программы по PATH (COM/EXE/BAT)
- `[x]` `FileSystem::resolve_for_open()` — открытие "голых" имён по search-каталогам
- `[x]` `find_first()` — перебор каталогов поиска (cwd → каталог программы → PATH)
- `[x]` Наследование окружения при EXEC: `resolve_child_env_seg()` читает PSP+0x2C родителя при env=0 (loader/mod.rs)
- `[ ]` Поиск окружения у родителя при EXEC (env=0 → наследование дерева env)

## 10. Рефакторинг — Разбивка файлов на модули

- `[x]` `alu.rs` → `alu/` с модулями `core`, `util`, `eval`, `inst`, `bcd`, `incdec`, `group`
- `[x]` `mov.rs` → `mov/` с модулями `rm`, `imm`, `str_ops`
- `[x]` `dos.rs` → `dos/` с модулями `dispatch`, `console`, `file_ops`, `dir`, `find`, `memory`, `exec`, `misc`
- `[x]` `video.rs` → `video/` с модулями `core`, `dac`, `planar`, `render`, `peripheral`, `vbe`, `types`
- `[x]` `dos.rs` → `dos/` + `filesystem/` разбит на `mod`, `types`, `path`, `device`, `find`, `search`; `loader/` на `com_loader`, `exe_loader`, `exe_header`, `psp`
- `[x]` `system.rs` → выделен `crate/dos/src/process.rs` (`ProcessManager`, `ParentSnapshot` — EXEC-жизненный цикл)
- `[x]` `bat/runner.rs` → подкаталог `bat/runner/` (`exec`, `commands`, `resolve`, `flow`, `types`, `util`)
- `[x]` `system.rs` (x86) → `system/` (`control`, `flags`, `interrupt`, `io`, `misc`, `regs`)
- `[ ]` `shift.rs` — разбить на подкаталог
- `[ ]` `bit.rs` — разбить на подкаталог
- `[ ]` `control.rs` (x86) — разбить на подкаталог
- `[ ]` `system.rs` (dos) — избавиться от god object (EXEC/снимки уже в process.rs; остальное — на усмотрение)

## 11. Сборка и CI

- `[ ]` Feature flags: `full`, `minimal`, `x86-only`, `with-audio`, `with-gui`
- `[ ]` Бенчмарки для ключественных инструкций x86
- `[ ]` GitHub Actions — build + test + lint (rustfmt + clippy)
- `[ ]` Fuzz-тестирование декодера ModRM и опкодов
- `[ ]` Интеграционные тесты: запуск реальных DOS .COM/.EXE через test.sh в CI
- `[!]` `RUST_MIN_STACK` для x86-тестов (контроль/group/system переполняют стек без него, см. ERRORS.md #38)

## 12. Документация

- `[x]` AGENTS.md — правила для агентов
- `[x]` GOALS.md — цели, приоритеты и этапы развития
- `[x]` TODO.md — дорожная карта
- `[x]` docs/architecture.md — описание архитектуры
- `[x]` docs/x86.md — список реализованных инструкций
- `[x]` docs/porting.md — как добавить поддержку новой периферии
- `[x]` docs/bus.md — описание трейтов Machine, Cpu, Peripheral, Display
- `[x]` docs/dos-api.md — полный список поддерживаемых INT 21h функций
- `[x]` docs/build.md — сборка, зависимости, конфигурация
- `[x]` README.md — главный README
- `[x]` DESIGN.md — дизайн-документ (включая BAT и EXEC)
- `[x]` SKILL.md — описание навыков
- `[x]` ERRORS.md — известные ошибки
- `[x]` MEMORIES.md — проектная память (решения, хронология)
- `[~]` docs/* синхронизация с последними изменениями (BAT, EXEC, env, PATH, CLI)
