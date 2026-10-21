# SKILL.md — Описание навыков для эмулятора rusn_dos

## Навыки разработчика

### 1. Разработка инструкций x86

Понимание архитектуры x86 (real mode, partial protected mode):
- ModRM (16/32-bit, SIB, segment defaults)
- Префиксы (0x66 operand size, 0x67 address size, 0xF2/F3 REP, 0xF0 LOCK, segment overrides)
- Вычисление флагов (CF, PF, AF, ZF, SF, OF) для арифметических, логических и сдвиговых операций
- Двухбайтовые опкоды 0x0F

### 2. Разработка DOS API

- INT 21h функции (AH=00h-6Ch): файловые операции, управление памятью (MCB), DTA, ввод/вывод
- INT 2Fh мультиплексные прерывания (DPMI, Windows)
- Механизм suspend/resume для неблокирующего ввода с клавиатуры

### 3. Разработка периферии

- Трейт Peripheral: port_read/write, mem_read/write, tick
- Трейт Display: update, poll_keys, pump_events (поддержание окна при suspend)
- Интеграция с Motherboard: регистрация портов, MMIO, вызов tick()

### 4. Добавление новой эмулируемой системы

- Новый крейт в `crate/`, зависимость от `bus` + `log`
- Реализация `Cpu` trait или предоставление `step()`
- Загрузка ROM/образов через `load_rom()` / `load_cd_image()`
- Интеграция с `DosSystem` или как standalone

### 5. Отладка

- `RUST_LOG=debug/trace` для логирования
- `X86Cpu::new_with_tracer("logopcode.txt")` для трассировки опкодов
- `--no-log` для отключения трассировки

## Типичные задачи

### Добавление нового опкода x86
1. Создать функцию в соответствующем файле `crate/x86/src/instructions/`
2. Добавить вызов в `executor.rs` (execute или execute_0f)
3. Написать тест в `crate/x86/tests/`
4. Обновить `docs/x86.md`

### Добавление новой функции DOS API
1. Добавить обработку в `crate/dos/src/interrupts/dos.rs` (или bios.rs/ems.rs)
2. Добавить в dispatch `interrupts/mod.rs` если новый вектор
3. Обновить TODO.md, topopcodes.md

### Добавление периферии
1. Создать файл в `crate/bus/src/peripherals/`
2. Реализовать `Peripheral` trait
3. Добавить в Motherboard: поле, инициализацию, проброс портов
4. Вызывать `tick()` в step_frame если нужно

## Сборка и тестирование

```bash
cargo build                               # Сборка
cargo clippy                              # Линтер
cargo test -p x86                         # Тесты CPU
cargo run Run test/simple.com --graphics  # Запуск DOS-программы
```
