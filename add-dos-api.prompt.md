# Prompt: Добавить новую функцию DOS API

## Контекст
Нужно реализовать новую функцию INT 21h или другого прерывания DOS/BIOS.

## Требования
1. Добавить обработку в `crate/dos/src/interrupts/dos/<module>.rs` (console.rs, file_ops.rs, dir.rs, find.rs, memory.rs, exec.rs, misc.rs — или bios.rs/ems.rs)
2. Подключить в dispatch: `crate/dos/src/interrupts/dos/dispatch.rs` (для INT 21h) или `crate/dos/src/interrupts/mod.rs` (для нового вектора)
3. Следовать шаблону существующих функций: извлечь параметры из регистров, выполнить операцию, установить результат
4. Использовать `set_carry_flag(sys, true/false)` для указания ошибки
5. Для ввода с клавиатуры: проверять `sys.machine.keyboard.pop_key()`, вызывать `sys.suspend_for_key(0x21)` если буфер пуст
6. При необходимости: установить IVT запись в `ivt.rs`

## После реализации
Обновить: `docs/dos-api.md`, `README.md` (если пункт в списке возможностей), `TODO.md` (отметить готово)

## Проверка
```bash
cargo build
cargo clippy
```
