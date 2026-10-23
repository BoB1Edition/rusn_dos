# Bus crate — трейты, примитивы и материнская плата

## Обзор

`crate/bus` — ядро абстракций эмулятора. Определяет трейты для CPU, памяти, периферии и дисплея, а также реализует материнскую плату с конкретной периферией.

```
crate/bus/src/
├── lib.rs          — реэкспорт всех публичных типов
├── machine.rs      — Machine trait (системная шина)
├── cpu.rs          — Cpu trait
├── peripheral.rs   — Peripheral trait
├── display.rs      — Display trait (окно/ввод)
├── memory.rs       — Memory (boxed-slice RAM)
├── motherboard.rs  — Motherboard (реализует Machine)
└── peripherals/    — PIC, PIT, клавиатура, VGA, шрифты
    ├── mod.rs
    ├── pic.rs
    ├── pit.rs
    ├── keyboard.rs
    ├── video.rs
    └── fonts_vga8x16.rs
```

## Трейты

### `Machine` (машина/системная шина)

CPU и периферия взаимодействуют с системой исключительно через `&mut dyn Machine`:

```rust
pub trait Machine {
    fn read_mem_u8(&self, addr: u32) -> u8;
    fn write_mem_u8(&mut self, addr: u32, val: u8);
    fn read_mem_u16(&self, addr: u32) -> u16 { /* lo | hi<<8 */ }
    fn write_mem_u16(&mut self, addr: u32, val: u16) { /* два байта */ }
    fn read_mem_u32(&self, addr: u32) -> u32 { /* два u16 */ }
    fn write_mem_u32(&mut self, addr: u32, val: u32) { /* два u16 */ }
    fn in_port(&mut self, port: u16) -> u8;
    fn out_port(&mut self, port: u16, val: u8);
    fn in_port16(&mut self, port: u16) -> u16;
    fn in_port32(&mut self, port: u16) -> u32;
    fn out_port16(&mut self, port: u16, val: u16);
    fn out_port32(&mut self, port: u16, val: u32);
    fn trigger_irq(&mut self, irq_line: u8);
    fn handle_software_interrupt(&mut self, vector: u8) -> bool;
    fn is_halted(&self) -> bool;
    fn halt(&mut self);
}
```

Методы `read_mem_u16/32`, `write_mem_u16/32`, `in_port16/32`, `out_port16/32` имеют реализации по умолчанию через последовательные однобайтовые вызовы (little-endian).

- `handle_software_interrupt` — CPU вызывает его при инструкции INT n. Если возвращает `true`, прерывание обработано внутренне; если `false` — CPU читает IVT и делает far jump на внешний обработчик.

### `Cpu` (процессор)

```rust
pub trait Cpu {
    fn step(&mut self, machine: &mut dyn Machine);
    fn is_halted(&self) -> bool;
    fn halt(&mut self);
}
```

Реализован в `crate/x86` как `X86Cpu`.

### `Peripheral` (периферийное устройство)

```rust
pub trait Peripheral {
    fn port_read(&mut self, port: u16) -> u8 { 0 }
    fn port_write(&mut self, port: u16, val: u8) {}
    fn mem_read(&mut self, addr: u32) -> u8 { 0 }
    fn mem_write(&mut self, addr: u32, val: u8) {}
    fn tick(&mut self) {}
}
```

Все методы опциональны. Периферия регистрируется в Motherboard и вызывается из `in_port`/`out_port`/`read_mem_u8`/`write_mem_u8`.

### `Display` (окно/ввод)

```rust
pub trait Display {
    fn is_open(&self) -> bool;
    fn size(&self) -> (usize, usize);
    fn poll_keys(&mut self) -> Vec<(u8, u8)>;
    fn update(&mut self, pixels: &[u32], width: usize, height: usize);
    fn pump_events(&mut self) {}
}
```

- `poll_keys()` — возвращает новые нажатия с прошлого вызова: `(scancode, ascii)`.
- `pump_events()` — опционально: перерисовывает последний кадр (используется при suspend для поддержания окна).

Реализован в `crate/gui` как `EmuWindow`.

### `Memory` (RAM)

```rust
pub struct Memory { data: Box<[u8]>, size: usize }

impl Memory {
    pub fn new(size: usize) -> Self;
    pub fn from_slice(data: Box<[u8]>) -> Self;
    pub fn read_u8(&self, addr: u32) -> u8;
    pub fn write_u8(&mut self, addr: u32, value: u8);
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn as_slice(&self) -> &[u8];
}
```

Обращается к памяти с контролем границ (лог на out-of-bounds, возврат 0 при чтении).

## Motherboard (материнская плата)

Реализует `Machine`, объединяя CPU, RAM и периферию:

Поля:

| Поле | Тип | Описание |
|------|-----|----------|
| `ram` | `Memory` | RAM |
| `video` | `VideoMemory` | VGA (текст/Mode13h/planar), DAC, CRTC, AC |
| `keyboard` | `Keyboard` | 8042 + BIOS буфер |
| `pic` | `Pic` | 8259 (master+slave) |
| `pit` | `Pit` | 8253 (3 канала, wall-clock stub) |
| `a20_enabled` | `bool` | A20 gate |
| `a20_command_pending` | `bool` | Флаг ожидания команды A20 |

Порты I/O:

| Порты | Устройство | Метод |
|-------|-----------|-------|
| 0x60, 0x64 | Keyboard | `keyboard.port_read/write` |
| 0x20, 0x21, 0xA0, 0xA1 | PIC | `pic.port_read/write` |
| 0x40–0x43 | PIT | `pit.port_read/write` |
| 0x3C0, 0x3C1 | VGA Attribute Controller | `video.port_read/write` |
| 0x3C2 | VGA Miscellaneous Output | `video.port_read/write` |
| 0x3C4, 0x3C5 | VGA Sequencer | `video.port_read/write` |
| 0x3C6–0x3C9 | VGA DAC | `video.port_read/write` |
| 0x3CE, 0x3CF | VGA Graphics Controller | `video.port_read/write` |
| 0x3D4, 0x3D5 | VGA CRTC | `video.port_read/write` |
| 0x3DA | VGA Input Status 1 | `video.port_read` |
| 0x92 | A20 gate | прямое чтение/запись |

MMIO (video):

| Диапазон (внутреннее смещение) | Режим | Назначение |
|-------|-------|------------|
| 0x18000–0x1FFFF | Text80x25 | Текстовый буфер B8000 |
| 0x00000–0x0FFFF | Planar | Плоскости VGA A0000 |
| 0x00000–0x0F9FF | Mode13h | Framebuffer A0000 |

### Прерывания

- `trigger_irq(irq_line)` — делегирует PIC; PIC устанавливает вектор в `pending`.
- `handle_software_interrupt(vector)` — возвращает `is_internal(vector)` (CS=0xF000).

### A20 gate

- `apply_a20_mask(addr)` — обнуляет бит 20 если `a20_enabled == false`.
- Управляется через порт 0x92 (бит 1) или keyboard controller (команда 0xD1).
- `a20_command_pending` — флаг, устанавливаемый при записи 0xD1 в 0x64, сбрасываемый при записи байта в 0x60.

## Периферия

### PIC (8259)

Два каскадированных контроллера (master 0x20/0x21, slave 0xA0/0xA1):

Поля:

- `master_mask`, `slave_mask` — маски IRQ.
- `irq_requested` — `[u8; 2]` флаги запросов.
- `pending_irq` — номер ожидающего прерывания.

Методы:

- `request_irq(irq)` — установить флаг запроса.
- `get_next_interrupt()` — взять следующий вектор (IRQ0→INT 08h).
- `acknowledge()` — EOI.

### PIT (8253)

Три канала, заглушка на wall-clock:

- `tick()` — всегда возвращает `true` (IRQ).
- `port_read(0x40–0x42)` — возвращает младшие миллисекунды.
- `channel[3]` — режимы каналов (не эмулируются).

### Keyboard (8042)

Поля:

- `buffer: VecDeque<u16>` — BIOS-буфер клавиш (`(scancode << 8) | ascii`).
- `shift_flags: u8` — состояние Shift/Ctrl/Alt (BDA 0x417).
- `command_byte: u8` — состояние контроллера.

Методы:

- `push_key(scancode, ascii)` — добавить в буфер, обновить shift_flags.
- `pop_key()` — извлечь, обновить BDA.
- `peek_key()` — посмотреть без извлечения.
- `keyboard_read(port)` / `keyboard_write(port, val)` — порты 0x60/0x64.

### VGA (VideoMemory)

Полноценная эмуляция VGA:

- Текстовый режим 80x25 (640×400).
- Mode 13h (320×200×8, 256 цветов).
- Планарные режимы: Mode 0Dh (320×200×16), 0Eh (640×200×16), 10h (640×350×16), 12h (640×480×16).
- DAC 18-bit палитра (256 цветов, 6 бит на канал).
- VGA Sequencer (0x3C4/0x3C5): маппинг страниц, режимы.
- VGA Graphics Controller (0x3CE/0x3CF): режимы чтения 0/1, записи 0–3, bit mask, ALU.
- VGA CRTC (0x3D4/0x3D5): start address, cursor, offset, scanlines.
- VGA Attribute Controller (0x3C0/0x3C1): palette mapping, blink, plane enable.
- VGA Input Status 1 (0x3DA): retrace тайминг, сброс flip-flop.

Подробнее — в `docs/architecture.md` (раздел Video System).

## Lib.rs — публичный API

```rust
pub use display::Display;
pub use machine::Machine;
pub use memory::Memory;
pub use peripheral::Peripheral;
pub use motherboard::Motherboard;
pub use cpu::Cpu;
pub use peripherals::pit::Pit;
pub use peripherals::video::{VideoMemory, VideoMode};
```
