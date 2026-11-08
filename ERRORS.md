# ERRORS.md — Найденные ошибки и проблемные места

> Проверка по состоянию кода на 06.11.2026 (коммит e34ed4c «memory, mcb fix» + незакоммиченные правки process.rs/system.rs).
> Пути и номера строк даны "как найдено" и могут сдвинуться при правках.

## Статус предыдущего отчёта (03.11.2026)

- ✅ **ИСПРАВЛЕНО #5** — `resolve_offset()` теперь используется в `crate/x86/src/instructions/mov/imm.rs:127`.
- ✅ **ИСПРАВЛЕНО #12** — `cf` в INC/DEC передаётся в `compute_flags_u32/u16` и больше не является мёртвой переменной.
- ✅ **ИСПРАВЛЕНО #16** — поле `pattern` в `SearchContext` уже не существует (осталось только в этом разделе для истории).
- ✅ **ИСПРАВЛЕНО #1 (частично)** — RCL/RCR переписаны на `u128` в `shift.rs` (5a75460); паники `1u64 << 64` больше нет.
- ⚠️ **НЕ ИСПРАВЛЕНО** остальное; см. актуальные пункты ниже.

## Критические (CRITICAL)

### 1. ~~UB в RCL/RCR: `1u64 << 64` при 32-bit rotate-through-carry~~ — ИСПРАВЛЕНО (5a75460)

- **Файл:** `crate/x86/src/instructions/shift.rs`
- **Severity:** CRITICAL (была)
- **Описание:** Раньше при `width == 32` переменная `wide = 64`, и `(1u64 << wide) - 1` — UB (паника `attempt to shift left with overflow`). Теперь RCL/RCR используют `u128` (`bits as u128`, маска `(1u128 << n) - 1`) — UB отсутствует.
- **Воспроизведение:** `0x66 0xD3 0xD0` (RCL eax, cl) — больше не паникует.

### 2. UB в SHLD/SHRD: `>> (32 - imm)` при `imm = 0`

- **Файл:** `crate/x86/src/instructions/bit.rs:16,31,52,67,88,103,124,139`
- **Severity:** CRITICAL
- **Описание:** Условие `imm > 31` / `shift > 31` пропускает `imm = 0`. Тогда `(32 - 0) = 32`, и `u32 >> 32` — UB (паника в debug). Аналогично для 16-битного варианта: `u16 >> 16` при `imm = 0`.
- **Затрагивает:** `shld_imm`, `shld_cl`, `shrd_imm`, `shrd_cl` — все 4 функции, обе ширины.
- **Исправление:** Использовать `wrapping_shr(32 - imm)`/`wrapping_shl` или ветку `if imm == 0 { src }`.

## Высокие (HIGH)

### 3. Bounds check: `text_buffer[idx]` без валидации cursor_row/cursor_col

- **Файл:** `crate/dos/src/interrupts/bios.rs:98` (INT 10h/AH=08h) и `:400` (`tty_write`)
- **Severity:** HIGH
- **Описание:** `idx = cursor_row as usize * 80 + cursor_col as usize` считается без проверки диапазона. Если guest напрямую пишет в CRTC регистры (0x0E/0x0F) без `sync_cursor_from_crtc()`, значения могут выйти за `text_buffer` (16384) — паника.
- **Исправление:** Заменить на `.get(idx).copied().unwrap_or(0)`.

### 4. PIC — цепочка прерываний мёртва

- **Файл:** `crate/bus/src/peripherals/pic.rs`
- **Severity:** HIGH
- **Описание:** `Pic::get_next_interrupt()` нигде не вызывается. Таймер (IRQ0) доставляет INT 08h напрямую через `raise_interrupt()`. Поля `master_irr`/`master_isr`/`slave_irr`/`slave_isr` остаются мёртвым кодом.
- **Исправление:** Либо подключить в цикл материнской платы, либо удалить.

### 5. `run_window` — заглушка при запуске без подкоманды

- **Файл:** `src/main.rs:141-143`
- **Severity:** MEDIUM (паника убрана, команда не реализована)
- **Описание:** `Commands::Win` и запуск без подкоманды вызывают `run_window()` — заглушка `Ok(())`. Запуск `rusn_dos` без подкоманды молча завершается с кодом 0, окно не открывается.
- **Исправление:** Реализовать `run_window` (запуск конфига без программы) или убрать команду `Win`.

### 6. Дублирование расчёта даты/високосного года (2 копии)

- **Файлы:**
  - `crate/dos/src/filesystem/find.rs:189` (`system_time_to_dos_datetime`)
  - `crate/dos/src/interrupts/dos/misc.rs:169` (`get_system_date`)
- **Severity:** HIGH
- **Описание:** Алгоритм `epoch seconds → (year, month, day)` повторён в двух местах почти дословно. Любое исправление бага в одном не применяется к другому.
- **Исправление:** Вынести в общую функцию в новый модуль `crate/dos/src/datetime.rs`.

### 7. `Peripheral` trait — вся диспетчеризация мёртва

- **Файл:** `crate/bus/src/peripheral.rs`
- **Severity:** HIGH
- **Описание:** `peripheral.tick()`, `mem_read()`, `mem_write()` никогда не вызываются Motherboard; I/O порты вызываются напрямую (`keyboard.port_read()` и т.д.).
- **Исправление:** Либо перейти на трейт-диспетчеризацию, либо удалить неиспользуемые методы.

### 8. `inhibit_interrupts` сбрасывается в конце той же инструкции

- **Файл:** `crate/x86/src/cpu.rs:194-196`
- **Severity:** HIGH
- **Описание:** Поле `inhibit_interrupts` (подавление прерываний после MOV SS / POP SS) выставляется в `mov_sreg_rm16` (mov/imm.rs:43) и `pop_ss` (stack.rs:60), но `X86Cpu::step()` сбрасывает его в самом конце **той же** инструкции. Инвариант x86 «прерывания подавлены до начала следующей инструкции» нарушен — `tick_timer()` никогда не видит флаг выставленным.
- **Воспроизведение:** `cargo test -p x86 --test mov_test` → `mov_sreg_and_ss_inhibit` FAILED; `cargo test -p x86 --test stack_test` → `segment_push_pop_and_pop_ss_inhibits` FAILED.
- **Исправление:** Не сбрасывать в конце `step()` — сбрасывать перед проверкой в `tick_timer`/`raise_interrupt` (т.е. флаг должен жить до завершения следующей инструкции). Это regress-баг, которого не было при 5ad1eb2^ (введён в 5ad1eb2 «teletype»).

## Средние (MEDIUM)

### 9. Небезопасный `unwrap()` в системе suspend/resume

- **Файл:** `crate/dos/src/system.rs:292`
- **Severity:** MEDIUM
- **Описание:** `self.suspended.take().unwrap().vector` — если `suspended == None` при `key == Some`, будет паника. Инвариант хрупкий.
- **Исправление:** `match self.suspended.take() { Some(s) => self.dispatch(s.vector), None => { /* лог */ } }`.

### 10. Прямой вывод в stdout в production коде

- **Файлы:**
  - `crate/dos/src/interrupts/dos/console.rs:125` — `println!()`
  - `crate/dos/src/interrupts/bios.rs:405-408` — `print!()` в `tty_write`
- **Severity:** MEDIUM
- **Описание:** Вывод минуя логгер; в оконном режиме не виден, в консольном засоряет.

### 11. Смешанные типы ошибок

- **Severity:** MEDIUM
- **Описание:** `Result<T, Box<dyn Error>>` в main.rs/app.rs, `Result<T, String>` в filesystem/runner, вообще без Result в обработчиках прерываний.
- **Исправление:** Единый тип ошибки для проекта.

### 12. Дублирование read/write helper'ов в group.rs

- **Файл:** `crate/x86/src/instructions/group.rs:419-454`
- **Severity:** MEDIUM
- **Описание:** `read16/32`, `write8/16/32` дублируют `read_rm*`/`write_rm*` из alu/util.
- **Исправление:** Использовать общие функции из `alu::util`.

### 13. `INT 21h AH=48h` c `BX=0` обрабатывается неверно

- **Файлы:** `crate/dos/src/interrupts/dos/memory.rs:9-11`, `crate/dos/src/mcb.rs:59-99`
- **Severity:** MEDIUM
- **Описание:** По DOS, `AH=48h BX=0` означает «выделить самый большой доступный блок». Здесь `paragraphs = 0`: `mcb::allocate` попадает в `mcb.size >= 0` (всегда true), при `size > 1` разрезает блок на дегенеративный нулевой блок (`owner=0x0001, size=0`) и возвращает сегмент без данных. BX при этом не заполняется максимальным размером.
- **Исправление:** В `allocate_memory_handler` обрабатывать BX=0 отдельно: `ax=max, cf=0, bx=max`.

### 14. `modify` grow возвращает неверный «максимальный размер» в Err

- **Файл:** `crate/dos/src/mcb.rs:208-219`
- **Severity:** MEDIUM
- **Описание:** При `mcb.signature == LAST` или невалидной подписи возвращается `Err(mcb.size)` — только размер текущего блока, без учёта следующего свободного. `memory.rs:51-55` ставит AX=0x08 и BX=max, но max занижен относительно того, что вернул бы `max_available_at`.
- **Исправление:** Всегда возвращать `Err(max_available_at(...))`.

### 15. Чрезмерно публичный API

- **Severity:** MEDIUM
- **Описание:**
  - `crate/dos/src/filesystem/types.rs` — `DeviceKind`, `FileHandle` — `pub` → `pub(crate)`
  - `crate/dos/src/filesystem/path.rs` — `resolve_device()` — `pub` → `pub(crate)`
  - `crate/dos/src/filesystem/find.rs` — `system_time_to_dos_datetime()` — `pub` → `pub(crate)`
  - `crate/x86/src/tracer.rs:60` — `struct NullTracer` — `pub` → `pub(crate)`
  - `crate/dos/src/consts.rs:3-8` — все `pub const` — `pub` → `pub(crate)`
  - `crate/x86/src/instructions/alu/core.rs:151,157` — `pub enum Width`, `pub enum Op` — `pub` → `pub(crate)`
  - `crate/bus/src/memory.rs:6,20` — `struct Memory`, `fn from_slice()` — `pub` → `pub(crate)`

## Низкие (LOW) — warnings компилятора

### 16. Неиспользуемый импорт `MCB_SIGNATURE_LAST`

- **Файл:** `crate/dos/src/loader/psp.rs:2`
- **Исправление:** В `psp.rs` используется только `MCB_SIGNATURE_NON_LAST` (в `build_environment_block`), а `MCB_SIGNATURE_LAST` — нет. Убрать из импорта (warning подтверждён `cargo build`).

### 17. `base_val = 0u32` — присвоение, перезаписываемое до чтения

- **Файл:** `crate/x86/src/modrm.rs:85,185`
- **Исправление:** `let mut base_val;`.

### 18. Неиспользуемый параметр `dh` в `scale_into_rect`

- **Файл:** `crate/bus/src/peripherals/video/render.rs:115`
- **Исправление:** `_dh` или использовать. Функция также имеет `clippy::too_many_arguments` (10/7).

### 19. Неиспользуемые константы `ENV_SEGMENT` (2 шт.)

- **Файлы:** `crate/dos/src/loader/com_loader.rs:29`, `crate/dos/src/loader/exe_loader.rs:79`
- **Исправление:** Убрать (окружение собирается через `build_environment_block`, константа не используется).

### 20. Мёртвый код в `src/app.rs` и `src/main.rs`

- `src/app.rs:248` — `fn init_drivers()` никогда не вызывается (заменена на `make_drivers()`).
- `src/main.rs:23` — флаг `--mount` объявлен, но нигде не читается.
- `src/main.rs:3` — неиспользуемый импорт `log::info` (в `main.rs` используется только `log::debug!`).
- **Исправление:** Удалить `init_drivers`; либо подключить `cli.mount`, либо убрать флаг; убрать импорт.

### 21. ~~Мёртвые поля/функции в незакоммиченных правках~~ — УТВЕРЖДЕНО ИСПОЛЬЗОВАНИЕ

- **Файлы:** `crate/dos/src/system.rs` (`init_sysvars()`, `sysvars_segment`, `sysvars_offset`), `crate/dos/src/process.rs` (`current_psp`)
- **Статус 06.11.2026:** WIP стал реальностью:
  - `init_sysvars()` вызывается из `loader/mod.rs:55` (`load_executable()`), сегмент хранится в `sysvars_segment/offset` и читается в INT 21h/AH=52h (dispatch.rs:137-147).
  - `current_psp` читается в INT 21h/AH=50h/51h/62h (dispatch.rs) и используется как владелец MCB в `memory.rs` (AH=48h).
- Снять пункт либо переформулировать в «проверить необходимость pub».

### 22. `init_memory_map()` — мёртвый код

- **Файл:** `crate/dos/src/mcb.rs:36`
- **Описание:** `init_memory_map` нигде не вызывается — все загрузчики вручную строят цепочку MCB (`prog_mcb`/`free_mcb` в com_loader.rs/exe_loader.rs). Удалить или подключить.

## Полный список из отчёта 03.11, требующий повторной сверки (часть — устарела)

Пункты с прежней нумерацией (до реорганизации этого документа). Нумерация идёт от старого файла ERRORS.md.

- #14 (чрезмерный pub API) — актуально, см. пункт 15 выше.
- #22 (лишние касты) — по-прежнему в `bit.rs:30,66,102,138,183`, `file_ops.rs:125`, `bios.rs:482,502`.
- #23 (`needless_range_loop`) — `dac.rs:5,44`, `planar.rs:51,171`, `file_ops.rs:102`.
- #24 (`manual_memcpy`) — `dac.rs:41-43`.
- #25 (`manual_is_multiple_of`) — 10 мест: `filesystem.rs` (после дробления — `filesystem/find.rs:…`), `misc.rs:184,194`, `flags.rs:63`, `core.rs:323,359`, `system.rs` и т.д. (`cargo clippy` насчитал 10).
- #26 (`identity_op`) — `planar.rs:48`, `registers.rs:95`, `misc.rs:110`.
- #27 (`collapsible_if`) — `system.rs:275-285` по-прежнему.
- #28 (платформенные заглушки) — 6 крейтов JVM/NES/SNES/MegaDrive/Saturn/Dreamcast с `step()/load_rom()/load_class()` и `log::info!("...not yet implemented")`.
- #29 (`leave()` — только 16-bit) — **УСТАРЕЛО**: `leave()` теперь обрабатывает 32-битный случай (control.rs:269-279) — добавлено в «91b10fa». Пункт можно закрыть.
- #30 (`mov_rm16_sreg`/`mov_sreg_rm16` — только 16-bit) — по-прежнему только 16-битные формы в `mov/imm.rs:8-54`.
- #31 (BSR/BSF и AAA/AAS отсутствуют, INT1/WAIT — заглушки) — подтверждено: `executor.rs` не содержит 0x37/0x3F/0F BC/0F BD; `int1` в `interrupt.rs:58` ставит `halted=true`; `wait` в `misc.rs:16` — no-op.
- #32 (в `bus` и `dos` нет тестов) — подтверждено: `cargo test -p bus` → 0 тестов, `cargo test -p dos` → 0 тестов.

## Новые/изменившиеся проблемы в EXEC / MCB (коммиты 93605c5, e34ed4c + незакоммичены)

### 33. EXEC: не обнуляются Entry-регистры ребёнка

- **Файлы:** `crate/dos/src/process.rs:130-137`, `crate/dos/src/loader/com_loader.rs:167-186`, `crate/dos/src/loader/exe_loader.rs:320-339`
- **Severity:** MEDIUM
- **Описание:** При загрузке ребёнка через `load_in_place` устанавливаются CS/IP/SS/SP/DS/ES/FS/GS/DTA, но **AX/BX/CX/DX/SI/DI/BP не обнуляются** — ребёнок видит регистры родителя. По DOS при запуске программы регистры AX=BX=CX=DX=SI=DI=BP=0.
- **Воспроизведение:** `TESTEXEC.COM` → `[7] Entry regs AX BX CX DX SI DI BP SP: 4B00 0171 0000 0161 0000 0000 0000 FFFA` (AX=4B00 — код от EXEC родителя, BX/DX — мусор от родителя).
- **Исправление:** В обоих `load_in_place` обнулить AX/BX/CX/DX/SI/DI/BP перед запуском.

### 34. EXEC: TESTEXEC.ASM параметрический блок нестандартного размера

- **Файл:** `test/testexec.asm` / `app/test/testexec.asm` (PB_ENV объявлен как `DW 0, 0` — 8 байт вместо стандартных 2+2)
- **Severity:** LOW (баг теста, а не эмулятора)
- **Описание:** Стандартный блок параметров EXEC: `+0 env(dw), +2 cmd(off), +4 cmd(seg), +6 fcb1, +8 fcb1s, +0A fcb2, +0C fcb2s`. Перекрытия нет — тест заполняет команду вручную в `MAIN` (MOV PB_CMD), а не литералой в `.data`, поэтому фактический размер блока корректен; лишнее слово `DW 0,0` ничего не ломает, но нестандартно. `cmd=016D:0000` в логе соответствует корректно собранному блоку.
- **Исправление:** `DW 0` (одно слово) для соответствия спецификации.

### 35. EXE загрузка: `e_maxep` не влияет на распределение памяти

- **Файл:** `crate/dos/src/loader/exe_loader.rs` (`exec`: `mem_paragraphs = MAX_MCB_SEGMENT - PSP_SEGMENT`; `load_in_place`: `required_paragraphs = 0x10 + prog_paragraphs`)
- **Severity:** MEDIUM
- **Описание:** В `load_in_place` размер BSS = `e_minep * 16`, а рекомендуемый максимум распределения из `e_maxep` игнорируется; PSP:02 (размер блока) равен `required_paragraphs` вместо `min(e_maxep, свободно)`. Часть программ, рассчитывающих на `e_maxep`, могут вести себя неверно.
- **Исправление:** Учитывать `e_maxep` при вычислении `mem_paragraphs` и `required_paragraphs`.

### 36. MCB: `dump_chain` вызывается в обработчиках INT 21h при каждой ошибке

- **Файлы:** `crate/dos/src/interrupts/dos/memory.rs:26,39,54`
- **Severity:** LOW
- **Описание:** При любой ошибке AH=48/49/4A выводится полная цепочка MCB через `log::debug`. Это засоряет логи и неинформативно при штатных отказах. Оставить только на `log::trace`/`RUST_LOG=debug` по запросу.

### 37. `tick_timer` с `executed += 1` в ветке `suspended`

- **Файл:** `crate/dos/src/system.rs:293`
- **Severity:** LOW
- **Описание:** В ветке ожидания клавиши счётчик `executed` инкрементируется без выполнения инструкции — бюджет звездца (`INSTR_BUDGET == 50_000`) тратится попусту в ожидании ввода. Косметика.
- **Исправление:** Использовать отдельный счётчик кадров.

## X86 тесты: требуют `RUST_MIN_STACK`

### 38. Stack overflow x86-тестов (нехватка стека потока теста)

- **Severity:** LOW (инфраструктура)
- **Описание:** `cargo test -p x86` без `RUST_MIN_STACK` падает с `thread ... has overflowed its stack` в `control_test`, `group_test`, `system_test`: `Motherboard::new` аллоцирует полноразмерный `VideoMemory` (несколько сотен KB→MB) в стеке потока теста. `RUST_MIN_STACK=16777216 cargo test` решает проблему.
- **Исправление:** Задать больший стек (`#[ctor]` или `RUST_MIN_STACK`) в тестовом harness либо уменьшить инициализацию видеопамяти в тестовых платах.

### 39. Начальное состояние GDTR/IDTR не соответствует железу

- **Файл:** `crate/x86/src/cpu.rs:57-60` (`X86Cpu::new`: `gdtr_limit: 0`, `idtr_limit: 0`)
- **Severity:** LOW
- **Описание:** На реальном 80386+ после сброса GDTR.limit = 0xFFFF, GDTR.base = 0 (та же логика для IDTR). Здесь оба лимита инициализируются = 0. Как следствие `extended_test::lgdt_lidt_sgdt_smsw_live_on_cpu` падает: `SGDT [0x4000]` пишет лимит 0 вместо ожидаемого 0xFFFF (`read_mem_u16(0x4000) == 0xFFFF` не выполняется).
- **Исправление:** В `X86Cpu::new` задать `gdtr_limit: 0xFFFF`, `idtr_limit: 0xFFFF` (база 0).

## Итог

| Категория | Кол-во |
|-----------|--------|
| CRITICAL | 1 (SHLD/SHRD; RCL/RCR исправлен) |
| HIGH | 5 |
| MEDIUM | 9 |
| LOW | 12 |
| clippy стиль | ~65 (10 manual_is_multiple_of, 8 unnecessary_cast, 5 collapsible_if, 5 manual_div_ceil, 4 too_many_arguments …) |
| Частичная реализация | 8 |
| X86 тестов падает (без RUST_MIN_STACK) | 3 бинаря (control/group/system) |
| X86 тестов падает (реальные баги) | 3 теста в 2 проблемах (SGDT/GDTR, inhibit_interrupts ×2) |
