# ERRORS.md — Найденные ошибки и проблемные места

> Проверка по состоянию кода на 03.11.2026 (коммит «много хороших изменений»).
> Пути и номера строк даны "как найдено" и могут сдвинуться при правках.

## Критические (CRITICAL)

### 1. UB в RCL/RCR: `1u64 << 64` при 32-bit rotate-through-carry

- **Файл:** `crate/x86/src/instructions/shift.rs:165,185`
- **Severity:** CRITICAL
- **Описание:** При `width == 32` переменная `wide = 64`, и `(1u64 << wide) - 1` — undefined behavior (в Rust сдвиг на ≥ разрядности — UB, в debug mode паника). Ранее был заход в `if c == 0 -> return`, но ненулевой count при width=32 ставит `wide = 64` — и `mask` считается через `1u64 << 64`.
- **Воспроизведение:** Любая 32-битная инструкция RCL/RCR с ненулевым count.
- **Исправление:** `let mask = if wide == 64 { u64::MAX } else { (1u64 << wide) - 1 };`

### 2. UB в SHLD/SHRD: `>> (32 - imm)` при `imm = 0`

- **Файл:** `crate/x86/src/instructions/bit.rs:15,30,51,66,87,102,123,138`
- **Severity:** CRITICAL
- **Описание:** Условие `imm > 31` пропускает `imm = 0`. Тогда `(32 - 0) = 32`, и `u32 >> 32` — UB. Аналогично для 16-битного варианта: `u16 >> 16` при `imm = 0`.
- **Затрагивает:** `shld_imm`, `shld_cl`, `shrd_imm`, `shrd_cl` — все 4 функции, обе ширины (аналог для `<<` и 16-bit путей).
- **Исправление:** Использовать `wrapping_shr(32 - imm)`/`wrapping_shl` или `if imm > 0 { ... } else { ... }`.

### 3. Bounds check: `text_buffer[idx]` без валидации cursor_row/cursor_col

- **Файл:** `crate/dos/src/interrupts/bios.rs:98,400` (`tty_write` и INT 10h/AH=08h)
- **Severity:** HIGH
- **Описание:** `idx = cursor_row as usize * 80 + cursor_col as usize` считается без проверки диапазона. Если guest напрямую пишет в CRTC регистры (0x0E/0x0F) без вызова `sync_cursor_from_crtc()`, значения могут выйти за `text_buffer` (16384). Индексация `text_buffer[idx]` — паника.
- **Исправление:** Заменить на `.get(idx).copied().unwrap_or(0)` (и в `tty_write` для чтения атрибута).

## Высокие (HIGH)

### 4. PIC — цепочка прерываний мёртва

- **Файл:** `crate/bus/src/peripherals/pic.rs`
- **Severity:** HIGH
- **Описание:** `Pic::get_next_interrupt()` нигде не вызывается (единственное упоминание — само определение метода). Таймер (IRQ0) доставляет INT 08h напрямую через `raise_interrupt()`, минуя PIC. Поля `master_irr`/`master_isr`/`slave_irr`/`slave_isr` — мёртвый код.
- **Исправление:** Либо подключить `get_next_interrupt()` в цикл `step_frame()`, либо удалить мёртвый код.

### 5. `resolve_offset()` — мёртвый код

- **Файл:** `crate/x86/src/modrm.rs:167`
- **Severity:** HIGH
- **Описание:** Публичный метод `resolve_offset()` нигде не вызывается. Всегда используется `resolve_address()`.
- **Исправление:** Удалить или скрыть до востребования.

### 6. `todo!()` в `run_window` — паника при запуске без подкоманды

- **Файл:** `src/main.rs:142`
- **Severity:** HIGH
- **Описание:** `Commands::Win` вызывает `run_window()` с `todo!("run_window")`. Любой запуск `rusn_dos` без подкоманды (или `rusn_dos Win`) приводит к панике; `Ok(())` недостижим.
- **Исправление:** Реализовать `run_window` (запуск конфига без программы) или убрать команду `Win`.

### 7. Дублирование расчёта даты/високосного года (3 копии)

- **Файлы:**
  - `crate/dos/src/filesystem.rs:695,703` (`system_time_to_dos_datetime`)
  - `crate/dos/src/interrupts/dos/misc.rs:184,194` (`get_system_date`)
- **Severity:** HIGH
- **Описание:** Алгоритм `epoch seconds → (year, month, day)` повторён почти дословно в двух местах (а также в `get_system_time` — своя копия перевода времени). Любое исправление бага в одном не применяется к другому.
- **Исправление:** Вынести в общую функцию в новый модуль `crate/dos/src/datetime.rs`.

### 8. `Peripheral` trait — вся диспетчеризация мёртва

- **Файл:** `crate/bus/src/peripheral.rs`
- **Severity:** HIGH
- **Описание:** Трейт `Peripheral` реализован для `Pic`, `Pit`, `Keyboard`, `VideoMemory`, но:
  - `peripheral.tick()` никогда не вызывается Motherboard
  - `peripheral.mem_read()` / `peripheral.mem_write()` никогда не вызываются
  - I/O порты вызываются напрямую: `keyboard.port_read()`, `pic.port_read()` и т.д.
- **Исправление:** Либо перейти на трейт-диспетчеризацию, либо удалить неиспользуемые методы трейта.

## Средние (MEDIUM)

### 9. Небезопасный `unwrap()` в системе suspend/resume

- **Файл:** `crate/dos/src/system.rs:279`
- **Severity:** MEDIUM
- **Описание:** `self.suspended.take().unwrap().vector` — если `keyboard.has_key()` вернёт `true`, а `suspended == None`, будет паника. Инвариант хрупкий.
- **Исправление:** Заменить на `match self.suspended.take() { Some(s) => self.dispatch(s.vector), None => { /* лог ошибки */ } }`.

### 10. Прямой вывод в stdout в production коде

- **Файлы:**
  - `crate/dos/src/interrupts/dos/console.rs:125` — `println!()`
  - `crate/dos/src/interrupts/bios.rs:406` — `print!("\n")` + прочие `print!` в `tty_write`
- **Severity:** MEDIUM
- **Описание:** Прямой вывод в stdout, минуя логгер. В оконном режиме этот вывод не виден, в консольном — засоряет вывод (особенно `tty_write` при каждом символе).
- **Исправление:** Обернуть в `log::debug!`/`log::info!` или оставить только для консольного режима.

### 11. Смешанные типы ошибок

- **Severity:** MEDIUM
- **Описание:** `Result<T, Box<dyn Error>>` в `main.rs`/`app.rs`, `Result<T, String>` в `filesystem.rs`/`bat/runner.rs`, вообще без `Result` в обработчиках прерываний. Нет единого подхода к обработке ошибок.
- **Исправление:** Определить общий тип ошибки для проекта.

### 12. `cf` переменная в INC — никогда не читается

- **Файл:** `crate/x86/src/instructions/alu/incdec.rs:12,22`
- **Severity:** MEDIUM
- **Описание:** `let cf = old == 0xFFFF_FFFF;` (строка 12) и `let cf = old == 0xFFFF;` (строка 22) вычисляются, но затем CF восстанавливается из `old_cf`. Переменная `cf` не используется.
- **Исправление:** Убрать неиспользуемые `let cf` или перенести в `compute_flags_u32`/`compute_flags_u16`.

### 13. Дублирование read/write helper'ов в group.rs

- **Файл:** `crate/x86/src/instructions/group.rs:419-454`
- **Severity:** MEDIUM
- **Описание:** `read16()`, `read32()`, `write8()`, `write16()`, `write32()` в `group.rs` по сути дублируют `read_rm*` / `write_rm*` из `alu/util.rs`.
- **Исправление:** Использовать общие функции из `alu::util` в `group.rs`.

### 14. Чрезмерно публичный API

- **Severity:** MEDIUM
- **Описание:**
  - `crate/dos/src/filesystem.rs:629` — `enum DeviceKind` — `pub` → `pub(crate)`
  - `crate/dos/src/filesystem.rs:639` — `enum FileHandle` — `pub` → `pub(crate)`
  - `crate/dos/src/filesystem.rs:644` — `fn resolve_device()` — `pub` → `pub(crate)`
  - `crate/dos/src/filesystem.rs:686` — `fn system_time_to_dos_datetime()` — `pub` → `pub(crate)`
  - `crate/x86/src/tracer.rs:60` — `struct NullTracer` — `pub` → `pub(crate)`
  - `crate/dos/src/consts.rs:3-8` — все `pub const` — `pub` → `pub(crate)`
  - `crate/x86/src/instructions/alu/core.rs:151,157` — `pub enum Width`, `pub enum Op` — `pub` → `pub(crate)`
  - `crate/bus/src/memory.rs:6,20` — `struct Memory`, `fn from_slice()` — `pub` → `pub(crate)`

### 15. Неиспользуемый импорт `MCB_SIGNATURE_LAST`

- **Файл:** `crate/dos/src/loader/psp.rs:2`
- **Severity:** LOW (но warning компилятора)
- **Исправление:** Убрать `MCB_SIGNATURE_LAST` из импорта.

### 16. Поле `pattern` в `SearchContext` никогда не читается

- **Файл:** `crate/dos/src/filesystem.rs:28`
- **Severity:** LOW
- **Исправление:** Убрать поле `pattern` или использовать его.

### 17. `base_val = 0u32` — присвоение, перезаписываемое до чтения

- **Файл:** `crate/x86/src/modrm.rs:85,185`
- **Severity:** LOW (warning компилятора)
- **Исправление:** Просто `let mut base_val;` (без игнорируемого литерала 0).

### 18. Неиспользуемый параметр `dh` в `scale_into_rect`

- **Файл:** `crate/bus/src/peripherals/video/render.rs:115`
- **Severity:** LOW (warning компилятора — параметр не читается)
- **Исправление:** `_dh: usize` или определить, для чего он нужен.

## Низкие (LOW) — clippy и стиль

### 19. Unreachable expression после `todo!`

- **Файл:** `src/main.rs:143`
- **Исправление:** Убрать `Ok(())` после `todo!("run_window")` (недостижимо).

### 20. Неиспользуемый импорт `log::info`

- **Файл:** `src/main.rs:3`
- **Исправление:** Убрать импорт (в `main.rs` используется только `env_logger::init()`).

### 21. Мёртвый код в `src/app.rs`

- **Файл:** `src/app.rs:160` — `fn init_drivers()` не вызывается (заменена на `make_drivers()`); флаг `--mount` в `src/main.rs:23` объявлен, но не используется.
- **Исправление:** Удалить `init_drivers()`; либо подключить `cli.mount`, либо убрать флаг.

### 22. Лишние касты

- `crate/x86/src/instructions/bit.rs:30,66,102,138,183` — `src.wrapping_shl(imm as u32) as u16`, `bit as u16` — избыточные касты
- `crate/dos/src/interrupts/dos/file_ops.rs:125` — `offset_high as i32`/`offset_low as i32` при уже `i32`
- `crate/dos/src/interrupts/bios.rs:482,502` — `cx as u16` при уже `u16`

### 23. `needless_range_loop` (4+ мест)

- `crate/bus/src/peripherals/video/dac.rs:5,44`
- `crate/bus/src/peripherals/video/planar.rs:51,171`
- `crate/dos/src/interrupts/dos/file_ops.rs:102`

### 24. `manual_memcpy`

- **Файл:** `crate/bus/src/peripherals/video/dac.rs:41-43`
- Заменить второй цикл на `p[16..32].copy_from_slice(&STD)`.

### 25. `manual_is_multiple_of` (12+ мест)

- `crate/dos/src/filesystem.rs:695,703`
- `crate/dos/src/interrupts/dos/misc.rs:184,194`
- `crate/x86/src/flags.rs:63`
- `crate/bus/src/peripherals/video/core.rs:323,359`
- `crate/dos/src/system.rs:275`

### 26. `identity_op`

- `crate/bus/src/peripherals/video/planar.rs:48` — `& 0xFF` избыточен после `as u8`
- `crate/x86/src/registers.rs:95` — `self.bx() >> 0` — сдвиг на 0
- `crate/dos/src/interrupts/dos/misc.rs:109` — `buf_off + 0x00`

### 27. `collapsible_if`

- **Файл:** `crate/dos/src/system.rs:275-285` (вложенные `if` внутри suspend-ветки)

## Частично реализованные методы

### 28. 6 платформенных заглушек

- `crate/jvm/src/lib.rs` — JVM заглушка
- `crate/nes/src/lib.rs` — NES заглушка
- `crate/snes/src/lib.rs` — SNES заглушка
- `crate/megadrive/src/lib.rs` — Mega Drive заглушка
- `crate/saturn/src/lib.rs` — Saturn заглушка
- `crate/dreamcast/src/lib.rs` — Dreamcast заглушка

Все содержат `step()`, `load_rom()`, `load_class()` и т.д. с `log::info!("...not yet implemented")`.

### 29. `leave()` — только 16-bit

- **Файл:** `crate/x86/src/instructions/control.rs:269-273`
- 32-bit (с префиксом 0x66) не обрабатывается.

### 30. `mov_rm16_sreg` / `mov_sreg_rm16` — только 16-bit

- **Файл:** `crate/x86/src/instructions/mov/imm.rs:8-54`
- Не обрабатывают 32-битный размер с префиксом 0x66.

### 31. BSR/BSF и AAA/AAS отсутствуют, INT1/WAIT — заглушки

- **Файл:** `crate/x86/src/executor.rs` (нет `0x37` AAA, `0x3F` AAS, `0F BC/BD` BSF/BSR)
- `crate/x86/src/instructions/system.rs:459` — `int1()` ставит `halted = true` с логом (не выполняет INT 1)
- `crate/x86/src/instructions/system.rs:107` — `wait()` — no-op (не эмулирует FSTSW/FNSTSW)
- **Замечание:** `0xA1`/`0xA3` — НЕ dead code, реализованы в `mov/imm.rs` и вызываются через `mov::mov_ax_address`/`mov_address_ax`.

## Тесты

### 32. В крейтах `bus` и `dos` нет тестов

- `crate/bus` — 0 unit-тестов, 0 doc-тестов
- `crate/dos` — 0 unit-тестов, 0 doc-тестов
- Все тесты только в `crate/x86` (56 `#[test]`).

## Итог

| Категория | Кол-во |
|-----------|--------|
| CRITICAL | 2 |
| HIGH | 6 |
| MEDIUM | 6 |
| LOW | 13 |
| clippy стиль | ~60 |
| Частичная реализация | 9 |
