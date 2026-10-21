# Prompt: Добавить новую периферию

## Контекст
Нужно добавить новое периферийное устройство в эмулятор.

## Требования
1. Создать файл в `crate/bus/src/peripherals/<name>.rs`
2. Реализовать `Peripheral` trait из `crate::Peripheral`:
   - `port_read()` / `port_write()` для портов I/O
   - `mem_read()` / `mem_write()` для MMIO (если нужно)
   - `tick()` для периодических обновлений
3. Добавить `pub mod <name>;` в `crate/bus/src/peripherals/mod.rs`
4. Добавить поле в `Motherboard` в `motherboard.rs`
5. Инициализировать в `Motherboard::new()`
6. Пробросить порты в `in_port()` / `out_port()`
7. Вызывать `tick()` в `DosSystem::step_frame()` если нужно

## Проверка
```bash
cargo build
cargo clippy
```
