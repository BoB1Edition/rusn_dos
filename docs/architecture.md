# Architecture — rusn_dos

## Overview

rusn_dos is a multi-system emulator written in Rust. Its primary target is IBM PC-compatible DOS emulation with an x86 (80386+) CPU core. Additional emulation targets (NES, Mega Drive, SNES, Saturn, Dreamcast, JVM) are scaffolded but not yet implemented.

```
┌─────────────────────────────────────────────────────┐
│                   rusn_dos (binary)                  │
│  src/main.rs  ─  src/app.rs  ─  config.toml          │
├─────────────────────────────────────────────────────┤
│  dos crate       DOS API, loaders, FS, EMS, MCB      │
│  x86 crate       80386 CPU decoder/executor           │
│  bus crate       Machine, Cpu, Peripheral, Memory     │
│  gui crate       minifb window, keyboard input        │
│  nes/megadrive/...  stubs                            │
└─────────────────────────────────────────────────────┘
```

## Crate Layout

### `bus` — Core Traits and Hardware

Defines abstract interfaces and concrete motherboard peripherals.

| Module | Contents |
|--------|----------|
| `machine.rs` | `Machine` trait — memory, ports, interrupts, halt |
| `cpu.rs` | `Cpu` trait — `step()`, `is_halted()`, `halt()` |
| `memory.rs` | `Memory` — boxed-slice RAM, bounds-checked access |
| `peripheral.rs` | `Peripheral` trait — `port_read/write`, `mem_read/write`, `tick` |
| `display.rs` | `Display` trait — window, key polling, pixel buffer |
| `motherboard.rs` | `Motherboard` — `Memory` + `Keyboard` + `VideoMemory` + `Pic` + `Pit`, implements `Machine` |
| `peripherals/` | `pic.rs`, `pit.rs`, `keyboard.rs`, `video.rs`, `fonts_vga8x16.rs` |

**Trait `Machine`** — the system bus that CPU uses to access memory, I/O ports, and interrupts:

```rust
pub trait Machine {
    fn read_mem_u8(&self, addr: u32) -> u8;
    fn write_mem_u8(&mut self, addr: u32, val: u8);
    fn in_port(&mut self, port: u16) -> u8;
    fn out_port(&mut self, port: u16, val: u8);
    fn trigger_irq(&mut self, irq_line: u8);
    fn handle_software_interrupt(&mut self, vector: u8) -> bool;
    fn is_halted(&self) -> bool;
    fn halt(&mut self);
}
```

Default implementations provide `read_mem_u16/32`, `write_mem_u16/32`, `in_port16/32`, `out_port16/32`.

### `x86` — CPU Core

Full 80386+ real-mode CPU implementation.

| Module | Contents |
|--------|----------|
| `registers.rs` | `Registers` — EAX..EDX, ESI..EDI, EBP, ESP, EIP, CS/DS/ES/SS/FS/GS, EFLAGS |
| `flags.rs` | Flag constants, `compute_flags_u8/16/32()`, `compute_logical_flags_*()` |
| `cpu.rs` | `X86Cpu` struct, prefix state, instruction fetch, `Cpu` trait impl |
| `modrm.rs` | ModRM decoder with 16/32-bit addressing, SIB, segment defaults |
| `executor.rs` | `execute(opcode)` and `execute_0f(opcode)` dispatch |
| `tracer.rs` | Opcode logging to file |
| `instructions/` | 10 instruction category modules |

### `dos` — DOS Layer

Connects x86 CPU to DOS environment.

| Module | Contents |
|--------|----------|
| `system.rs` | `DosSystem` — motherboard + CPU + filesystem + EMS, `step()`, `step_frame()` |
| `ivt.rs` | IVT initialization (0x00000-0x003FF), internal handler routing |
| `interrupts/` | `dos.rs` (INT 21h), `bios.rs` (INT 10h/12h/15h/16h/1Ah/08h), `ems.rs` (INT 67h) |
| `loader/` | COM loader, EXE loader (MZ), executable detection |
| `filesystem.rs` | Multi-drive filesystem, `DiskDriver`, file handles, find-first/next |
| `mcb.rs` | DOS Memory Control Block allocator |
| `consts.rs` | Memory size constants |

### `gui` — Window and Input

minifb-based implementation of the `Display` trait. Keyboard mapping from minifb `Key` to DOS (scancode, ASCII) pairs.

## Interrupt Handling Flow

```
Software Interrupt (INT n)
  │
  ├── CPU pushes flags, CS, IP
  ├── CPU disables IF, TF
  ├── Reads handler CS:IP from IVT[vector*4]
  │
  └─► DosSystem::dispatch() [via CpuBus]
        │
        ├── INT 21h → dos.rs
        ├── INT 10h → bios.rs
        ├── INT 67h → ems.rs
        ├── INT 2Fh → dos.rs
        ├── INT 08h → bios.rs (PIT → chains to INT 1Ch)
        ├── INT 16h → bios.rs
        ├── INT 15h → bios.rs
        ├── INT 12h → bios.rs
        ├── INT 1Ah → bios.rs
        └── other → far call via IVT
```

## Main Loop (`DosSystem::step_frame`)

```
step_frame(display):
  budget = 50,000 instructions
  while budget > 0 AND not halted:
    step()              // 1 CPU instruction + dispatch pending SWI
    tick_timer()        // every 65536 steps → IRQ0
    every 256 steps: poll display for keys
    if VRAM dirty: render frame, return early
```

## Memory Map (Real Mode)

| Range | Content |
|-------|---------|
| 0x00000-0x003FF | IVT (256 vectors × 4 bytes) |
| 0x00400-0x004FF | BIOS Data Area |
| 0x00500-0x9FFFF | Conventional Memory (DOS) |
| 0xA0000-0xBFFFF | Video RAM (VGA) |
| 0xC0000-0xEFFFF | ROM/Expansion |
| 0xF0000-0xFFFFF | BIOS area (internal handlers) |
| 0x100000-0xEFFFFF | Extended Memory (XMS) |
