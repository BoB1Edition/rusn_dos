# AGENTS.md — Правила и соглашения для агентов, работающих с кодом

## Проект: rusn_dos — Мультисистемный эмулятор на Rust (beta)

### Архитектура

- `crate/` — workspace с крейтами: `x86`, `bus`, `dos`, `gui`, `emu_config`, `nes`, `megadrive`, `snes`, `jvm`, `saturn`, `dreamcast`
- `crate/bus` — шина, трейты `Machine`, `Cpu`, `Display`, `Peripheral`, `Memory`, примитивы; материнская плата `Motherboard` с периферией (PIC, PIT, клавиатура, VGA)
- `crate/x86` — эмуляция CPU 80386+: регистры, флаги, ModRM (16/32-bit + SIB), декодер (`execute`/`execute_0f`), все инструкции
- `crate/dos` — DOS-оболочка: загрузчики EXE/COM, BAT-интерпретатор, файловая система (многодисковая + устройства DOS + поиск по PATH), IVT, MCB, EMS, SYSVARS, EXEC (AH=4Bh/4Dh), прерывания (INT 08h–67h)
- `crate/gui` — графическое окно (minifb), клавиатура (scancode → DOS)
- `crate/emu_config` — типы конфигурации (Resolution, MachineConfig, VideoConfig, SoundConfig, DriverConfig, CpuConfig, EnvironmentConfig)
- `src/` — точка входа (clap CLI, subcommands Run/Win), конфигурация (config.toml), монтирование дисков, stdin_bridge (crossterm raw-mode), маршрутизация .BAT/.CMD
- максимальное избегание длинных более 300-400 строк файлов исходников, длинные файлы допустимы только если разбить файл логически совсем не возможно (`./crate/bus/src/peripherals/fonts_vga8x16.rs`) или на время беты, группировка по логике в отдельные каталоги
- максимальное избегание god object где только можно
- использование макросов для избегания дублирования кода приветствуется
- переход на альфу предполагает перевод каждого процессора\сопроцессора на свой поток, то есть видео карта — поток, аудио — отдельный поток, таймер — отдельный поток, но перед этим проводится анализ, действительно ли это нужно
- в альфа режим могут переводиться как само приложение в целом, так и отдельные крейты, модули
- в альфу не может быть переведён крейт или модуль, который зависит от крейта/модуля, находящегося в бете
- из альфы можно переводить в бету не модифицируя код, а исправляя файлы ALFA.md, PREALFA.md и BETA.md
- в проекте находятся специальные файлы ALFA.md, PREALFA.md и BETA.md
- BETA.md содержит все крейты и модули, находящиеся в бета
- PREALFA.md — все крейты и модули, предложенные для перевода в альфу
- ALFA.md — все крейты и модули, находящиеся в альфе

### Стиль кода

- rustfmt с настройками по умолчанию (edition 2024)
- `clippy` — все предупреждения должны быть устранены
- Комментарии случайны в бета, в альфа режиме комментарии должны быть максимально полными и понятными они должны быть основой документации, cargo doc должен собрать документацию о каждой функции, поле и макросе проекта, так что бы после прочтения доки вопросов не было почему так, как так и как это можно поменять.
- Все ошибки обрабатывать через `Result<T, Box<dyn Error>>` и логи
- `todo!()` и `unreachable!()` допустимы только в бета версии как заглушки на время разработки
- Вектора с capacity: `Vec::with_capacity(n)` при известном размере

### Разработка инструкций x86

- Каждая группа инструкций в отдельном файле `crate/x86/src/instructions/`
- Крупные файлы инструкций разбиваются на подкаталоги: `alu/` → `core.rs`, `util.rs`, `eval.rs`, `inst.rs`, `bcd.rs`, `incdec.rs`, `group.rs`, `mod.rs`; `mov/` → `rm.rs`, `imm.rs`, `str_ops.rs`, `mod.rs`.
- `crate/dos/src/interrupts/dos/` разбит на: `dispatch.rs`, `console.rs`, `file_ops.rs`, `dir.rs`, `find.rs`, `memory.rs`, `exec.rs`, `misc.rs`, `mod.rs`.
- `crate/dos/src/filesystem/` разбит на: `mod.rs`, `types.rs`, `path.rs`, `device.rs`, `find.rs`, `search.rs`.
- `crate/dos/src/loader/` разбит на: `mod.rs` (детекция COM/EXE, `load_executable`, PSP/command-tail хелперы, `init_sysvars`), `com_loader.rs`, `exe_loader.rs`, `exe_header.rs`, `psp.rs`.
- `crate/dos/src/process.rs` — `ProcessManager` (EXEC): `start_child`/`finish_child`, `ParentSnapshot`, `current_psp`, тracer ребёнка.
- `crate/dos/src/bat/runner/` разбит на: `exec.rs`, `commands.rs`, `resolve.rs`, `flow.rs`, `types.rs`, `util.rs`, `mod.rs`.
- `crate/bus/src/peripherals/video/` разбит на: `core.rs`, `dac.rs`, `planar.rs`, `render.rs`, `peripheral.rs`, `vbe.rs`, `types.rs`, `mod.rs`.
- ModRM разбирается через `ModRm::from_byte()` + `resolve_address()` / `resolve_offset()`
- Префиксы 0x66/0x67 обрабатываются через `cpu.prefixes.has_operand_size / has_address_size`
- Флаги: `compute_flags_u8/16/32()` для арифметики, `compute_logical_flags_*()` для логики
- INC/DEC не меняют CF — сохранять через `old_cf = flags & CF`
- 0x0F двухбайтовые опкоды — через `execute_0f()`
- `fetch_u8/u16/u32` автоматически инкрементируют EIP и пишут в `instruction_bytes` для трассировки

### Разработка BAT-интерпретатора

- Модуль `crate/dos/src/bat/` — `mod.rs` + `runner/` (подкаталог: `exec.rs`, `commands.rs`, `resolve.rs`, `flow.rs`, `types.rs`, `util.rs`) — BAT-интерпретатор уровня COMMAND.COM
- Каждая дочерняя программа получает свежий `DosSystem` через `loader::load_executable()`, состояние оболочки (CWD, текущий диск, окружение, errorlevel, echo) живёт в Rust (`ShellState`) и переносится в ребёнка перед стартом и читается обратно после завершения
- `BatConfig` — конфигурация запуска: `max_depth`, `no_stdin_keys`, `no_log`; `PumpFn` — колбэк неблокирующего pump'а (передан в `run_bat`)
- `Runner::exec_line()` — построчный разбор, встроенные команды (echo, cd, cls, goto, if errorlevel, if exist, set, pause, call, exit, rem, смена диска) в `commands.rs`/`flow.rs`; внешние (через `run_batch` для .BAT с проверкой глубины и `run_external` для .COM/.EXE) в `resolve.rs`/`exec.rs`
- Поиск команд в `resolve_command()`: текущий диск + CWD → PATH → программа; расширение добавляется только после имени (COM/EXE/BAT)
- Вход в BAT: `.bat`/`.cmd` интерпретируются в `src/app.rs` в ветке `is_bat()` (как в консольном `App::run()`, так и в графическом `App::run_with_graphics()`)
- Глубина вложенности BAT контролируется параметром `--max-call-depth` (1..=255, default 20); `Flow::Goto` возможен только вперёд и назад по меткам, `if_depth > 4` запрещён
- Дисплей передаётся по значению через `Option<&mut dyn Display>` — заимствование окна живёт только на время вызова (см. комментарии в `exec.rs` про явный re-borrow в `child_loop()`)

### Разработка для других систем

- Каждая система — отдельный крейт в `crate/`
- Должна реализовать базовый трейт `Cpu` или предоставить `step()`
- `load_rom()` / `load_cd_image()` / `load_class()` — загрузка контента
- `DosSystem` — основная интеграционная шина (x86 + DOS + будущие системы)

### Сборка и тестирование

```bash
cargo build
cargo test -p x86       # модульные тесты CPU
cargo run Run test/test1
cargo run Run test/test2 --graphics
cargo run Run test/build.bat --graphics
cargo run Run app/XCOM/GO.COM --graphics -w C:\\XCOM
RUST_LOG=debug cargo run Run test/test1
```

- Тестовые DOS-программы живут в `test/` (исходники `.asm`) и в `app/test/` (собранные `.com`/`.exe`/`.bat`, см. `test/build.bat`)
- Скрипт прогона: `test.sh` (сборка + запуск всех целевых программ)
- Новые BAT-тесты: `test/chain.bat` (params, SET, errorlevel, CALL), `test/fail.bat` (errorlevel + goto + exit code), `test/build.bat` (TASM сборка в DOS), `test/recurce.bat` — рекурсия на глубину через `--max-call-depth`

### Соглашения

- EMS 4.0 (INT 67h) — в `crate/dos/src/interrupts/ems.rs`
- IVT инициализируется в `crate/dos/src/ivt.rs` (вектора с CS=0xF000 — внутренние)
- EXEC (INT 21h/AH=4Bh/AL=00 + AH=4Dh): `exec_program()` в `crate/dos/src/interrupts/dos/exec.rs`, жизненный цикл ребёнка — `start_child()`/`finish_child()` в `crate/dos/src/process.rs` (снимок родителя в `ParentSnapshot`, копирование command tail и FCB в PSP ребёнка, восстановление при завершении, тracer ребёнка в `logopcode_<имя>.txt`)
- VBE — VESA BIOS Extensions: режимы 101h (640x480x256), 103h (800x600x256), 105h (1024x768x256), 112h (24-bit), LFB на 0xE0000000, банковский доступ через A0000
- Железо: PIC (8259), PIT (8253), клавиатура (8042), VGA (CRTC, sequencer, DAC, attribute controller, graphics controller) — в `crate/bus/src/peripherals/`
- Устройства DOS: `tty_write()` — пишет и в VRAM и в stdout (используется INT 21h/AH=02h,06h,09h,0Ah, INT 10h/AH=0Eh в текстовом режиме)
- Механизм suspend/resume для неблокирующего ввода: `SuspendedCall` в `system.rs` (re-dispatch через `dispatch(vector)`, не stack-unwind)
- EXEC (INT 21h/AH=4Bh/AL=00 + AH=4Dh): `exec_program()` в `crate/dos/src/interrupts/dos/exec.rs`, жизненный цикл ребёнка — `start_child()`/`finish_child()` в `crate/dos/src/process.rs` (снимок родителя в `ParentSnapshot`, копирование command tail и FCB в PSP ребёнка, восстановление при завершении)
- SYSVARS/List of Lists: `init_sysvars()` в `crate/dos/src/system.rs` (сегмент `60:0026`), INT 21h/AH=52h возвращает `sysvars_segment:offset`
- Окружение DOS: `EnvironmentConfig` в `crate/emu_config` (переменные PATH/COMSPEC/PROMPT), строится в PSP через `build_environment_block()` в `crate/dos/src/loader/psp.rs`; поиск программ по PATH в `resolve_program_path()` (src/app.rs), `resolve_for_open()` (filesystem.rs), `path_entries()` (emu_config)
- Ошибки: `Result<T, Box<dyn Error>>` в `src/`, `Result<T, String>` в `filesystem.rs`; коды выхода программы — `u8` (`App::run()`/`run_with_graphics()` возвращают `Result<u8, Box<dyn Error>>`, `main` завершается с кодом программы или 1 при ошибке эмулятора)

### Приоритетные направления

1. x86 real mode — корректность опкодов и флагов (RCL/RCR/SHLD/SHRD UB, AAA/AAS, BSF/BSR)
2. Загрузчики EXE/COM — полная поддержка MZ и COM (e_maxep, обнуление Entry-регистров — ERRORS.md #33/#35)
3. DOS API — INT 21h/AH=00h-6Ch (остатки: 39h/3Ah/59h/71h), INT 2Fh, INT 33h, EXEC 4B00h
4. BAT-интерпретатор — echo/cd/cls/goto/if/set/pause/call/exit/rem, CALL :label на Этапе 4 не поддерживается
5. VGA — планарные режимы (0Dh/0Eh/10h/12h), DAC 18-bit палитра, sequencer/graphics controller, INT 10h (AH=00h–0Fh, 12h), курсор
6. Звук — реализация PC Speaker, SB, SB16, SB pro, UltraSound, Pro Audio Spectrum, Pro Audio Spectrum 16
7. Поддержка NES
8. Поддержка sega megadrive
9. VBE (VESA BIOS Extensions) — режимы 101h/103h/105h/112h, LFB, банкинг
10. Документация
11. Стабилизация
