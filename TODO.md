# План миграции: удаление `libs/dos_core`

**Цель**: перенести всю функциональность из `libs/dos_core` в `crate/*`, сохранив обратную совместимость, и удалить каталог `libs/`.

**Архитектура после миграции**:

```
crate/bus/   — Machine, Memory, Peripheral, Cpu, Motherboard (шина, переиспользуема)
crate/x86/   — X86Cpu (только CPU, ничего не знает о DOS)
crate/dos/   — DosSystem (Motherboard + X86Cpu + DOS-слой: загрузчик, ФС, прерывания, MCB)
crate/gui/   — окно minifb, ввод, рендеринг
src/         — CLI + App, владеет циклом эмуляции
```

`DosSystem` **содержит** `Motherboard` и `X86Cpu`, а не наследует их. Это позволяет в будущем:
- `DosSystem` → DOS-эмулятор
- `NesSystem` → эмулятор NES (другой CPU, та же шина или своя)
- `SegaSystem` → эмулятор Sega
- `JavaSystem` → JVM на виртуальной машине

---

## Этап 0. Подготовка

- [x] **0.1** Добавить `crate/dos/Cargo.toml` зависимости: `bus`, `x86`, `log`, `minifb` (последняя — для gui, не для dos)
- [x] **0.2** Добавить `crate/gui/Cargo.toml` зависимости: `minifb`, `bus`
- [x] **0.3** Убедиться, что `cargo check --workspace` проходит (на пустых заготовках dos/gui)

---

## Этап 1. Расширение `crate/bus`

Периферия шины должна покрывать все порты, которые были в `libs/dos_core/src/instructions/system.rs` (IN/OUT) и `cpu/run.rs` (опрос клавиатуры окна).

- [x] **1.1** **Таймер 8253/8254** в `bus/src/peripherals/pit.rs`:
  - Три канала (0–2), порты 0x40–0x42
  - Channel 0 — IRQ0 (системный таймер)
  - Channel 1 — refresh (заглушка)
  - Channel 2 — PC speaker (заглушка)
  - `tick()` на каждый вызов синхронизации
- [x] **1.2** **VGA-порты** (заглушки) в `bus/src/peripherals/video.rs`:
  - Порты 0x3C2, 0x3C4, 0x3C5, 0x3CE, 0x3CF — возвращают 0
  - Методы `port_read`/`port_write` у `VideoMemory`
  - `VideoMemory.set_mode()` принимать u8 (0x03 = Text80x25, 0x13 = Mode13h)
- [x] **1.3** **PIC**: расширить до двух каскадных PIC (мастер 0x20–0x21, слейв 0xA0–0xA1):
  - Сейчас только мастер. Добавить slave_mask, slave_irr, slave_isr
  - Порты 0xA0/0xA1
- [x] **1.4** **A20** через системный порт 0x92 (уже есть в `Motherboard::out_port`)
- [x] **1.5** `Motherboard::trigger_irq()`: если IF включён в CPU — CPU должен обрабатывать прерывания. На данном этапе оставить заглушку (прерывания в новом цикле будут через `call_interrupt`, а не через PIC → CPU)
- [x] **1.6** `handle_software_interrupt()` в `Motherboard`: пока всегда `false` (обработчики DOS ещё в libs)

---

## Этап 2. Перенос файловой системы и MCB в `crate/dos`

- [x] **2.1** Скопировать `libs/dos_core/src/filesystem.rs` → `crate/dos/src/filesystem.rs`
  - Сигнатуры `read_u8`/`write_u8` заменить на `&dyn Machine` (через `read_mem_u8`/`write_mem_u8`)
  - `extract_filename` принимает `&dyn Machine` вместо `DosMachine`
- [x] **2.2** Скопировать `libs/dos_core/src/consts.rs` → `crate/dos/src/consts.rs`
- [x] **2.3** Скопировать `libs/dos_core/src/mcb.rs` → `crate/dos/src/mcb.rs`
  - Заменить `DosMachine` на `&dyn Machine`
- [x] **2.4** `crate check --workspace` должен проходить

---

## Этап 3. Видеосистема — слияние `dos_core::video` и `bus::VideoMemory`

- [x] **3.1** В `crate/bus/src/peripherals/video.rs`:
  - Добавить метод `render_text_to_pixels()` (скопировать реализацию из `libs/dos_core/src/video/mod.rs`)
  - Добавить `upscale_framebuffer()` и `scale_buffer()`
  - Добавить палитру VGA (из `libs/dos_core/src/video/mod.rs`)
  - Добавить `load_vga_palette()`
  - Шрифт `fonts_vga8x16.rs` + `get_fonts_vga8x16()`
- [x] **3.2** Шрифт — скопировать `libs/dos_core/src/video/fonts_vga8x16.rs` → `crate/bus/src/peripherals/fonts_vga8x16.rs`
- [x] **3.3** `cargo check --workspace` проходит

---

## Этап 4. Загрузчик COM/EXE в `crate/dos`

- [x] **4.1** Скопировать `libs/dos_core/src/loader/` → `crate/dos/src/loader/`
- [x] **4.2** `ComLoader::exec()` и `ExeLoader::exec()` вместо `DosMachine` создают `(Motherboard, X86Cpu)`:
  - Создают `Motherboard::new(DOS_MEMORY_SIZE)`
  - Записывают PSP, сегменты, стек, CS:IP через `machine.write_mem_u8/u16`
  - Пишут регистры в `cpu.registers` (CS, IP, SS, SP, DS, ES)
  - Инициализируют IVT (через `init_ivt` — см. этап 5)
- [x] **4.3** `loader::load_executable(path, no_log?)` возвращает `DosSystem` (новый тип из этапа 6)
- [x] **4.4** `cargo check --workspace` проходит

---

## Этап 5. IVT и обработчики прерываний в `crate/dos`

- [x] **5.1** Скопировать `libs/dos_core/src/ivt.rs` → `crate/dos/src/ivt.rs`
  - Вместо `DosMachine` — `&dyn Machine` (write_phys_u16)
- [x] **5.2** Скопировать `libs/dos_core/src/interrupts/` → `crate/dos/src/interrupts/`
  - bios.rs, dos.rs, ems.rs, mod.rs
- [x] **5.3** Заменить все обращения к `machine.registers.*` на обращение к регистрам CPU:
  - Обработчики получают `&mut X86Cpu` + `&mut dyn Machine`
  - `cpu.registers.ah()`, `machine.write_mem_u8(buffer_addr, byte)` и т.д.
- [x] **5.4** `DosSystem::handle_software_interrupt(vector)`:
  - Читает IVT: если handler_cs == 0xF000 — вызывает dispatch
  - dispatch направляет на `bios::handle_int10(cpu, machine)` и т.д.
  - Возвращает `true` (обработано) или `false` (нужен кадр IVT)
- [x] **5.5** `X86Cpu.step()` при INT (0xCD) вызывает `machine.handle_software_interrupt()`. Если `true` — не делает кадр FLAGS/CS/IP.
- [x] **5.6** `cargo check --workspace` проходит

### Список переносимых обработчиков:

| Вектор | Функция | Файл |
|--------|---------|------|
| INT 08h | IRQ0-таймер → вызов INT 1Ch | bios.rs |
| INT 10h | Установка режима, телетайп, статус | bios.rs |
| INT 12h | Размер low memory | bios.rs |
| INT 15h | A20, расширенная память, E820 | bios.rs |
| INT 16h | Клавиатура BIOS: ожидание/проверка/shift | bios.rs |
| INT 1Ah | Системное время (тики) | bios.rs |
| INT 20h | Завершение программы | dos.rs |
| INT 21h | DOS API: вывод, файлы, память, DTA | dos.rs |
| INT 2Fh | Мультиплекс: EMS check, DPMI, сеть | dos.rs |
| INT 67h | EMS LIM 4.0 | ems.rs |

---

## Этап 6. `DosSystem` — главный объект DOS-эмулятора

- [x] **6.1** Создать `crate/dos/src/system.rs`:

```rust
pub struct DosSystem {
    pub machine: Motherboard,
    pub cpu: X86Cpu,
    pub filesystem: FileSystem,
    pub first_mcb_segment: u16,
    // keyboard уже внутри machine; клавиатурный буфер для INT 16h — тоже там
}
```

- [x] **6.2** Реализовать `Machine for DosSystem` — делегировать `machine`:
  - `read_mem_u8` → `self.machine.read_mem_u8`
  - `write_mem_u8` → `self.machine.write_mem_u8`
  - `in_port` → `self.machine.in_port`
  - `out_port` → `self.machine.out_port`
  - `handle_software_interrupt` → dispatch из этапа 5
  - `is_halted` → `self.cpu.halted`
  - `halt` → `self.cpu.halted = true`
- [x] **6.3** `DosSystem::step_frame()` — один шаг эмуляции:
  1. Выполнить N инструкций через `cpu.step(self)` (пока `cpu.halted`)
  2. Опрос клавиатуры окна (если окно есть) — push_key в machine.keyboard
  3. Если video.dirty — рендеринг (вызов gui)
  4. Псевдо-таймер (каждые 65536 шагов → INT 08h, если IF)
- [x] **6.4** `cargo check --workspace` проходит

---

## Этап 7. `crate/gui` — рендеринг и ввод

- [x] **7.1** `crate/gui/src/lib.rs`:

```rust
pub struct EmuWindow {
    window: minifb::Window,
}

impl EmuWindow {
    pub fn new(title: &str, width: usize, height: usize) -> Self;
    pub fn is_open(&self) -> bool;
    pub fn get_size(&self) -> (usize, usize);
    /// Возвращает список новых нажатий (scancode, ascii)
    pub fn poll_keys(&self, prev: &HashSet<Key>) -> Vec<(u8, u8)>;
    pub fn update(&mut self, pixels: &[u32], w: usize, h: usize);
}
```

- [x] **7.2** Функция `minifb_to_dos(key) -> Option<(u8, u8)>` из `cpu/run.rs` переезжает в gui
- [x] **7.3** Функции рендеринга: `render_text`, `upscale_framebuffer`, `scale_buffer` — либо здесь, либо оставить в bus/video и вызывать оттуда

---

## Этап 8. Переключение `src/` на новую архитектуру

- [x] **8.1** `Cargo.toml` (корневой):
  - Заменить `dos_core = { path = "libs/dos_core" }` на `dos = { path = "crate/dos" }`, `x86`, `bus`, `gui`
- [x] **8.2** `src/app.rs`:
  - `App::run()`:
    1. `let mut ds = dos::loader::load_executable(program, no_log)?;`
    2. `self.init_drivers(&mut ds)?;` — монтирование дисков (ds.filesystem.add_driver)
    3. Главный цикл: `while !ds.cpu.halted { ds.step_frame(None); }`
  - `App::run_with_graphics()`:
    1. Создать `EmuWindow`
    2. В цикле: `ds.step_frame(Some(&mut window))`
    3. После шага: `window.update(...)`
- [x] **8.3** `run_window()` — пока оставить `todo!()`
- [x] **8.4** `cargo build` собирает бинарник без `libs/dos_core`
- [x] **8.5** `./test.sh` проходит (те же тестовые программы)

---

## Этап 9. Удаление `libs/`

- [x] **9.1** Удалить `libs/dos_core/` целиком
- [x] **9.2** Удалить `libs/` из `workspace.members` в корневом `Cargo.toml`
- [x] **9.3** Удалить `libs/dos_core/src/hybrid_run.rs` (никогда не был подключён)
- [x] **9.4** `cargo check --workspace` — без ошибок
- [x] **9.5** `cargo test -p x86` — тесты crate/x86 всё ещё проходят

---

## Чек-лист валидации

- `cargo check --workspace` без ошибок на каждом этапе
- `cargo test -p x86` зелёный
- `RUST_LOG=warn ./target/debug/rusn_dos run ./test/test1` — работает как раньше
- `./test.sh` — все тесты проходят
- Количество предупреждений не увеличилось (было ~40 на старте)
