# Changelog — rusn_dos

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- **BAT-интерпретатор уровня COMMAND.COM**: новый модуль `crate/dos/src/bat/` (`mod.rs` + `runner.rs`, ~850 строк). Встроенные команды — `echo`/`echo.`/`echo off|on`, `cd`/`chdir`, `cls`, `goto`, `if errorlevel N`, `if exist`, `set` (включая вывод списка), `pause`, `call` (внешний .BAT, depth-controlled), `exit [code]`, `rem`, смена диска (`C:`). Раскрытие `%0`–`%9` и `%VAR%` (`%%` → `%`). Маршрутизация `.BAT`/`.CMD` в `src/app.rs` (ветка `is_bat()`).
- **EXEC (INT 21h/AH=4Bh/AL=00) + AH=4Dh**: новый модуль `crate/dos/src/interrupts/dos/exec.rs` — `exec_program()` (парсит параметрический блок ES:BX, ставит `ExecRequest`, CF/AX проставляются после возврата ребёнка) и `get_return_code()`. Жизненный цикл ребёнка в `crate/dos/src/system.rs`: `start_child()`/`finish_child()`, снимок родителя в `ParentSnapshot` (память + CPU + DTA + break_check), копирование command tail и FCB в PSP ребёнка, поддержка вложенных EXEC через `parent_stack`. Тест: `test/testexec.asm`.
- **EnvironmentConfig**: новый тип в `crate/emu_config` — прозрачная обёртка `HashMap<String, String>` над секцией `[environment]` config.toml (PATH/COMSPEC/PROMPT). Методы `get()` (case-insensitive), `path_entries()` (дробит PATH на пары (диск, каталог)). `build_environment_block()` в `crate/dos/src/loader/psp.rs` теперь принимает `&EnvironmentConfig` и динамически вычисляет сегмент блока окружения (возвращает его вместо константы).
- **Поиск программ по PATH**: `App::resolve_program_path()` (src/app.rs) — если файл не найден как есть и не содержит точки, подставляются .COM/.EXE/.BAT и ищутся по каталогам PATH. В `crate/dos/src/filesystem.rs` добавлены `set_program_location()`, `set_search_path_from_env()`, `search_candidates()`, `resolve_for_open()` (для "голых" имён при INT 21h/AH=3Dh; режим создания — всегда в CWD), `find_first()` перебирает каталоги поиска, а не один каталог.
- **INT 21h/AH=33h** (`break_check_handler` в `misc.rs`): AL=00 get / AL=01 set / AL=02 swap break-check, AL=05 boot drive, AL=06 версия DOS 6.22.
- **`CpuConfig`**: тип в `emu_config` (name/type/features). **`MachineConfig::disable_change_flags`**: блокировка изменения флагов конфигурации из гостевых программ (используется INT 15h A20).
- **`DosSystem::program_terminated`, `exec_request`, `parent_stack`, `child_exit_code/type`, `break_check`, `environment`**: новые поля.
- **Экспорт кода выхода программы**: `App::run()` и `App::run_with_graphics()` возвращают `Result<u8, ...>`; `main` выходит с кодом программы (или 1 при ошибке эмулятора) через `process::exit()` (после `run_main()`, чтобы RAII-гарды raw-mode успели отработать).
- **Валидация `--max-call-depth`**: диапазон 1..=255 (ошибка иначе).
- **Тесты**: `test/chain.bat` (params, SET, errorlevel, CALL), `test/fail.bat` (errorlevel + goto + exit code), `test/build.bat` (TASM сборка в DOS, `cd \test` + `tasm.exe` + `tlink.exe`), `test/recurce.bat` (рекурсия через максимальную глубину CALL), `test/testexec.asm` (INT 21h/AH=4Bh). `test.sh` расширен BAT-прогонами.

### Changed

- **Загрузчики COM/EXE**: `load_executable()`, `ComLoader::exec()`, `ExeLoader::exec()`, `DosSystem::new()` принимают `&EnvironmentConfig`; `build_environment_block()` строит блок из переменных, а не хардкод-строки.
- **INT 21h/AH=00h и AH=4Ch**: теперь выставляют также `cpu.halted = true` (раньше только `program_terminated`).
- **`App::init_drivers()`** заменена на `make_drivers()` (диски для BAT-ветки) + монтирование по одному в `.run()`/`.run_with_graphics()`.
- **`test.sh`**: в прогоны добавлены `-w c:\\biing`, `-w c:\\xcom`, `-w c:\\test` (DOS workdir), прогоны `tlink.exe`, `build.bat`, `fail.bat`, `recurce.bat` (`-d 3` и `-d 300`), `chain.bat hello`, `grep.com ?`.
- **`config.toml`**: добавлена секция `[environment]` (PATH=«C:\;C:\TASM\BIN», COMSPEC, PROMPT) и второй диск `Z:` = `/tmp/mgk/`.

### Fixed

- **`DosSystem::new()`**: параметр `environment` обязателен — согласованы все места создания системы (загрузчики, BAT-интерпретатор, EXEC).

## [Previous Releases — see git log]
