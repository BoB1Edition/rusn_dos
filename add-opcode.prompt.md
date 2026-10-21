# Prompt: Добавить новый x86 опкод

## Контекст
Нужно добавить реализацию опкода/инструкции x86, который отсутствует в эмуляторе.

## Требования
1. Создать/изменить функцию в `crate/x86/src/instructions/<category>.rs`
2. Поддержать 8/16/32-bit варианты с учётом префикса `has_operand_size`
3. Корректно вычислять флаги (CF, PF, AF, ZF, SF, OF) через `compute_flags_*` или `compute_logical_flags_*`
4. ModRM разбирать через `ModRm::from_byte()`
5. Адрес памяти вычислять через `modrm.resolve_address(cpu, machine, addr32)`
6. Регистры читать/писать через `read_reg8/16/32`, `write_reg8/16/32`
7. Добавить вызов в `executor.rs` (execute или execute_0f)
8. Написать тест в `crate/x86/tests/`

## Проверка
```bash
cargo test -p x86
cargo clippy -p x86
```
