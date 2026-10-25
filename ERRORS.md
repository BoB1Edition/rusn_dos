# ERRORS.md — Найденные ошибки и проблемные места

## Критические (CRITICAL)

### 1. UB в RCL/RCR: `1u64 << 64` при 32-bit rotate-through-carry

- **Файл:** `crate/x86/src/instructions/shift.rs:165,185`
- **Severity:** CRITICAL
- **Описание:** При `width == 32` переменная `wide = 64`, и `(1u64 << 64) - 1` — undefined behavior (в Rust сдвиг на ≥ разрядности — UB, в debug mode паника).
- **Воспроизведение:** Любая 32-битная инструкция RCL/RCR с ненулевым count.
- **Исправление:** Добавить `if wide == 64 { u64::MAX } else { (1u64 << wide) - 1 }`.

### 2. UB в SHLD/SHRD: `>> (32 - imm)` при `imm = 0`

- **Файл:** `crate/x86/src/instructions/bit.rs:15,30,51,66,87,102,123,138`
- **Severity:** CRITICAL
- **Описание:** Условие `imm > 31` пропускает `imm = 0`. Тогда `(32 - 0) = 32`, и `u32 >> 32` — UB. Аналогично для 16-битного варианта: `u16 >> 16` при `imm = 0`.
- **Затрагивает:** `shld_imm`, `shld_cl`, `shrd_imm`, `shrd_cl` — все 4 функции, обе ширины.
- **Исправление:** Использовать `wrapping_shr(32 - imm)` или заменить на `if imm > 0 { ... >> (32 - imm) } else { 0 }`.

### 3. Bounds check: `text_buffer[idx]` без валидации cursor_row/cursor_col

- **Файл:** `crate/dos/src/interrupts/bios.rs:98,400`
- **Severity:** HIGH
- **Описание:** `idx = cursor_row * 80 + cursor_col`. Если guest напрямую пишет в CRTC регистры (0x0E/0x0F) без вызова `sync_cursor_from_crtc()`, значения `cursor_row`/`cursor_col` могут быть любыми `u8`. `cursor_row = 255` даёт `255 * 80 + 79 = 20479`, что за пределами `text_buffer` (16384). Паника.
- **Исправление:** Заменить на `sys.machine.video.text_buffer.get(idx).copied().unwrap_or(0)`.

## Высокие (HIGH)

### 4. PIC — цепочка прерываний мёртва

- **Файл:** `crate/bus/src/peripherals/pic.rs:39`
- **Severity:** HIGH
- **Описание:** `Pic::get_next_interrupt()` никем не вызывается. Таймер (IRQ0) доставляет INT 08h напрямую через `raise_interrupt()`, минуя PIC. Поля `master_irr`, `master_isr`, `slave_irr`, `slave_isr` — мёртвый код. Эмуляция PIC не работает как контроллер прерываний.
- **Исправление:** Либо подключить `get_next_interrupt()` в цикл `step_frame()`, либо удалить мёртвый код.

### 5. `resolve_offset()` — мёртвый код

- **Файл:** `crate/x86/src/modrm.rs:167`
- **Severity:** HIGH
- **Описание:** Публичный метод `resolve_offset()` нигде не вызывается. Всегда используется `resolve_address()`.
- **Исправление:** Удалить или закомментировать до востребования.

### 6. `todo!()` в `run_window` — паника при запуске без подкоманды

- **Файл:** `src/main.rs:117`
- **Severity:** HIGH
- **Описание:** `Commands::Win` вызывает `run_window()` с `todo!("run_window")`. Любой запуск без подкоманды приводит к панике. Также `config` не используется, `Ok(())` недостижим.
- **Исправление:** Реализовать `run_window` или убрать команду `Win`.

### 7. Дублирование расчёта даты/високосного года (3 копии)

- **Файлы:**
  - `crate/dos/src/filesystem.rs:597-620`
  - `crate/dos/src/interrupts/dos/misc.rs:182-216`
- **Severity:** HIGH
- **Описание:** Алгоритм `epoch seconds → (year, month, day)` повторён почти дословно в двух местах. Любое исправление бага в одном не применяется к другому.
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

- **Файл:** `crate/dos/src/system.rs:262`
- **Severity:** MEDIUM
- **Описание:** `self.suspended.take().unwrap().vector` — если `keyboard.has_key()` вернёт `true`, а `suspended == None`, будет паника. Инвариант хрупкий.
- **Исправление:** Заменить на `match self.suspended.take() { Some(s) => self.dispatch(s.vector), None => { /* лог ошибки */ } }`.

### 10. `println!` в production коде

- **Файлы:**
  - `crate/dos/src/interrupts/dos/console.rs:125` — `println!()`
  - `crate/dos/src/interrupts/bios.rs:406` — `print!("\n")`
- **Severity:** MEDIUM
- **Описание:** Прямой вывод в stdout, минуя систему логирования. В оконном режиме этот вывод не виден, в терминальном — засоряет вывод.
- **Исправление:** Заменить на `log::info!`/`log::debug!`.

### 11. Смешанные типы ошибок

- **Severity:** MEDIUM
- **Описание:** `Result<T, Box<dyn Error>>` в `main.rs`/`app.rs`, `Result<T, String>` в `filesystem.rs`, вообще без `Result` в обработчиках прерываний. Нет единого подхода к обработке ошибок.
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
  - `crate/dos/src/filesystem.rs:531` — `enum DeviceKind` — `pub` → `pub(crate)`
  - `crate/dos/src/filesystem.rs:542` — `enum FileHandle` — `pub` → `pub(crate)`
  - `crate/dos/src/filesystem.rs:547` — `fn resolve_device()` — `pub` → `pub(crate)`
  - `crate/x86/src/tracer.rs:60` — `struct NullTracer` — `pub` → `pub(crate)`
  - `crate/dos/src/consts.rs:3-8` — все `pub const` — `pub` → `pub(crate)`
  - `crate/x86/src/instructions/alu/core.rs:151,157` — `enum Width`, `enum Op` — `pub` → `pub(crate)`
  - `crate/bus/src/memory.rs:6` — `struct Memory` — `pub` → `pub(crate)`
  - `crate/bus/src/memory.rs:20` — `fn from_slice()` — `pub` → `pub(crate)`
  - `crate/dos/src/filesystem.rs:589` — `fn system_time_to_dos_datetime()` — `pub` → `pub(crate)`

### 15. Неиспользуемый импорт `MCB_SIGNATURE_LAST`

- **Файл:** `crate/dos/src/loader/psp.rs:3`
- **Severity:** LOW (но warning компилятора)
- **Исправление:** Убрать `MCB_SIGNATURE_LAST` из импорта.

### 16. Поле `pattern` в `SearchContext` никогда не читается

- **Файл:** `crate/dos/src/filesystem.rs:28`
- **Severity:** LOW
- **Исправление:** Убрать поле `pattern` или использовать его.

### 17. `base_val = 0u32` — присвоение, перезаписываемое до чтения

- **Файл:** `crate/x86/src/modrm.rs:85,185`
- **Severity:** LOW (warning компилятора)
- **Исправление:** Просто `let mut base_val;` или `let mut base_val: u32;`

### 18. Неиспользуемый параметр `dh` в `scale_into_rect`

- **Файл:** `crate/bus/src/peripherals/video/render.rs:115`
- **Severity:** LOW (warning компилятора)
- **Исправление:** `_dh: usize` или определить, для чего он нужен.

## Низкие (LOW) — clippy и стиль

### 19. Unreachable expression после `todo!`

- **Файл:** `src/main.rs:118`
- **Исправление:** Убрать `Ok(())` или убрать `todo!()`.

### 20. Неиспользуемый импорт `log::info`

- **Файл:** `src/main.rs:3`
- **Исправление:** Убрать импорт.

### 21. Лишние касты в `bit.rs`

- **Файл:** `crate/x86/src/instructions/bit.rs:30,66,102,138,183`
- `src.wrapping_shl(imm as u32) as u16` — каст `u16 → u16` избыточен.
- `bit as u16` где `bit` уже `u16`.

### 22. Лишние касты в `file_ops.rs`

- **Файл:** `crate/dos/src/interrupts/dos/file_ops.rs:125`
- `offset_high as i32` и `offset_low as i32` — `i32 → i32`.

### 23. Лишние касты в `bios.rs`

- **Файл:** `crate/dos/src/interrupts/bios.rs:482,502`
- `cx as u16` — `u16 → u16`.

### 24. `needless_range_loop` (4+ мест)

- `crate/bus/src/peripherals/video/dac.rs:5,44`
- `crate/bus/src/peripherals/video/planar.rs:51,171`
- `crate/dos/src/interrupts/dos/file_ops.rs:102`

### 25. `manual_memcpy`

- **Файл:** `crate/bus/src/peripherals/video/dac.rs:41-43`
- Заменить цикл на `p[16..32].copy_from_slice(&STD)`.

### 26. `manual_is_multiple_of` (12+ мест)

- `crate/dos/src/filesystem.rs:598` (6 шт.)
- `crate/dos/src/interrupts/dos/misc.rs:183,193` (6 шт.)
- `crate/x86/src/flags.rs:63`
- `crate/bus/src/peripherals/video/core.rs:323,359`
- `crate/dos/src/system.rs:275`

### 27. `identity_op`

- `crate/bus/src/peripherals/video/planar.rs:48` — `& 0xFF` избыточен после `as u8`
- `crate/x86/src/registers.rs:95` — `self.bx() >> 0` — сдвиг на 0
- `crate/dos/src/interrupts/dos/misc.rs:109` — `buf_off + 0x00`

### 28. `collapsible_if`

- **Файл:** `crate/dos/src/system.rs:275-285`

## Частично реализованные методы

### 29. 6 платформенных заглушек

- `crate/jvm/src/lib.rs` — JVM заглушка
- `crate/nes/src/lib.rs` — NES заглушка
- `crate/snes/src/lib.rs` — SNES заглушка
- `crate/megadrive/src/lib.rs` — Mega Drive заглушка
- `crate/saturn/src/lib.rs` — Saturn заглушка
- `crate/dreamcast/src/lib.rs` — Dreamcast заглушка

Все содержат `step()`, `load_rom()`, `load_class()` и т.д. с `log::info!("...not yet implemented")`.

### 30. `leave()` — только 16-bit

- **Файл:** `crate/x86/src/instructions/control.rs:269-273`
- 32-bit (с префиксом 0x66) не обрабатывается.

### 31. `mov_rm16_sreg` / `mov_sreg_rm16` — только 16-bit

- **Файл:** `crate/x86/src/instructions/mov/imm.rs:8-54`
- Не обрабатывают 32-битный размер с префиксом 0x66.

## Тесты

### 32. В крейтах `bus` и `dos` нет тестов

- `crate/bus` — 0 unit-тестов, 0 doc-тестов
- `crate/dos` — 0 unit-тестов, 0 doc-тестов
- Все тесты только в `crate/x86` (56 тестов).

## Итог

| Категория | Кол-во |
|-----------|--------|
| CRITICAL | 2 |
| HIGH | 6 |
| MEDIUM | 10 |
| LOW | 12+ |
| clippy стиль | ~60 |
| Частичная реализация | 8 |
