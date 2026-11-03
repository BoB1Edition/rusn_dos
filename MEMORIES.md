# MEMORIES.md — Проектная память rusn_dos

Хранилище ключевых решений, договорённостей и хронологии проекта для быстрого восстановления контекста агентами и разработчиками. Актуализируется вместе с остальной документацией.

## Как устроен этот файл

- **Решения и договорённости** — принципы, которым следуем независимо от реализации.
- **Хронология** — что когда менялось относительно дорожной карты.
- **Ловушки** — известные подводные камни и почему код именно такой.

## Ключевые решения

### R1. Деления на альфа/бета через файлы ALFA.md / BETA.md / PREALFA.md
Весь код в бете. Перевод в альфу — только по плану через файлы `*.prealfa.md` / `*.alfa.md` (модуль или крейт) + обновление трёх статусных файлов. Из альфы в бету можно без изменения кода. Крейт в альфе не может зависеть от крейта в бете.

### R2. Отсутствие god object и длинных файлов
Файлы >300-400 строк делим на подкаталоги по логике (`x86/instructions/alu/`, `dos/interrupts/dos/`, `bus/peripherals/video/`). Дублирование убираем макросами. То же относится к новому `bat/runner.rs` (~850 строк) — это бета-компромисс, требует будущего разбиения.

### R3. Ошибки — через `Result<T, Box<dyn Error>>` + логи
В `src/` и загрузчиках. Исключение: `filesystem.rs` и BAT-интерпретатор по историческим причинам используют `Result<T, String>` (см. ERRORS.md #11 про унификацию).

### R4. Неблокирующий ввод — re-dispatch вместо stack-unwind
`SuspendedCall` хранит только вектор. При клавише прерывание вызывается заново `dispatch(vector)`. Это упрощение, принципиально отличное от сохранения CS:IP:SP:SS.

### R5. BAT-интерпретатор — вариант B: свежий `DosSystem` на каждую дочернюю программу
В отличие от черновика плана (один `DosSystem` + перезагрузка программы), реализация пошла по варианту B: каждая COM/EXE получает новый `DosSystem` через `loader::load_executable()`, а состояние оболочки живёт в Rust `ShellState`. Итог: изоляция, отсутствие утечек PSP/памяти между программами, перенос CWD/окружения явный.

### R6. EXEC — снимок родителя в `ParentSnapshot`
Ребёнок загружается в тот же `DosSystem`, но перед подменой память+CPU+параметры родителя сохраняются в `parent_stack` (поддержка вложенного EXEC). Восстановление — в `finish_child()`.

### R7. Окружение DOS — конфигурируемое (`EnvironmentConfig`)
Переменные (PATH/COMSPEC/PROMPT) задаются в `[environment]` config.toml, в PSP строятся динамически с вычислением сегмента. Не хардкодить новые переменные в загрузчике — только через конфиг.

### R8. Вывод DOS — в VRAM и stdout одновременно
`tty_write()` пишет и в видеопамять и в stdout: программы должны быть "видимы" и в консоли, и в графическом окне.

### R9. Документация синхронна с кодом
При любом функциональном изменении обновляются: AGENTS.md, CHANGELOG.md, DESIGN.md, ERRORS.md, GOALS.md, README.md, SKILL.md, TODO.md, MEMORIES.md + `docs/`. Документы на русском (основной язык), детали почему/как поменять — обязательны.

## Хронология

### 2026-11-03 — GOALS.md
Создан `GOALS.md` — цели, приоритеты (P1–P6), векторы и этапы (корректное ядро → играбельность → железо и платформы → v1.0). README/TODO/SKILL/MEMORIES перелинкованы на новый документ.

### 2026-11-03 — «много хороших изменений» (aibranch)
Основная тема — BAT-интерпретатор + EXEC + окружение:

- `crate/dos/src/bat/` — `runner.rs` (852 строки), `BatConfig`, `ShellState`, `run_bat()`. Команды echo/cd/cls/goto/if errorlevel/if exist/set/pause/call/exit/rem/смена диска. `%0-%9`/`%VAR%`/`%%`/`@`.
- `start_child()`/`finish_child()`/`ParentSnapshot`/`ExecRequest` — EXEC в `system.rs`.
- `interrupts/dos/exec.rs` — INT 21h AH=4Bh/4Dh.
- `EnvironmentConfig` в `emu_config`, секция `[environment]` в config.toml, `build_environment_block()` возвращает сегмент.
- Поиск программ по PATH: `App::resolve_program_path()`, `FileSystem::{set_program_location, set_search_path_from_env, search_candidates, resolve_for_open}`, `find_first()` перебирает каталоги.
- INT 21h AH=33h (break check, boot drive, версия DOS).
- Конец выхода программы: `App::run()/run_with_graphics()` → `Result<u8, ...>`, `main` через `process::exit`.
- Тесты: chain.bat, fail.bat, build.bat, recurce.bat, testexec.asm; test.sh расширен.

### 2026-10-25 — VGA-подсистема, разбиение файлов
- VGA: CRTC (0x3D4/0x3D5), attribute controller (0x3C0/0x3C1), DAC, текстовый буфер 0x4000, render_text/render_mode13/render_planar.
- SVGA/VBE: 101h/103h/105h/112h, LFB 0xE0000000, банкинг.
- Разбиение `alu.rs`, `mov.rs`, `dos.rs`, `video.rs` на подкаталоги.
- stdin_bridge (crossterm raw mode), CpuConfig, disable_change_flags.

## Ловушки (почему код так выглядит)

- **RCL/RCR** — `wide = 64` при 32-bit, сдвиг `1u64 << 64` — UB. Живой баг (ERRORS.md #1), аккуратно при правках.
- **SHLD/SHRD** — `imm = 0` не покрывается `if imm > 31`, `>> 32` — UB (ERRORS.md #2).
- **BAT display borrow** — дисплей в `runner.rs` передаётся по значению `Option<&mut dyn Display>` через всю цепочку; заимствование окна живёт только на время вызова (`child_loop`), чтобы BorrowChecker не растягивал borrow на итерации.
- **`--no-stdin-keys` + интерактивная программа** — `NO_STDIN_KEYS_ERR` в `stdin_bridge.rs`; в BAT-ветке своя константа `NO_STDIN_MSG` (дублирование, ждёт унификации).
- **`tty_write`** — индекс `cursor_row * 80 + cursor_col` без bounds-check в `bios.rs`; писать в CRTC напрямую может увести за буфер.
- **`file_ops.rs` / `misc.rs`** — отдельные копии перевода epoch→дата; правки дат делать в обеих (или выносить в `datetime.rs`).
- **Диск C:** — хардкод во многих местах (`get_current_drive`, `get_current_directory` при DL=0). При введении других дисков искать все такие места.
