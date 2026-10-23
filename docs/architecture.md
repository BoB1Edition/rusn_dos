# Architecture — rusn_dos

## Overview

rusn_dos is a multi-system emulator written in Rust. Its primary target is IBM PC-compatible DOS emulation with an x86 (80386+) CPU core. Additional emulation targets (NES, Mega Drive, SNES, Saturn, Dreamcast, JVM) are scaffolded but not yet implemented.

```
┌──────────────────────────────────────────────────────────┐
│                    rusn_dos (binary)                       │
│  src/main.rs  ─  src/app.rs  ─  config.toml               │
├──────────────────────────────────────────────────────────┤
│  dos crate       DOS API, loaders, FS, EMS, MCB           │
│  x86 crate       80386 CPU decoder/executor                │
│  bus crate       Machine, Cpu, Peripheral, Memory          │
│  gui crate       minifb window, keyboard input             │
│  emu_config      Config types (Resolution, Video, Sound…)  │
│  nes/megadrive/…  stubs                                   │
└──────────────────────────────────────────────────────────┘
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
| `peripherals/` | `pic.rs` (8259), `pit.rs` (8253), `keyboard.rs` (8042), `video.rs` (VGA — text/planar/mode13h, DAC, sequencer, graphics controller), `fonts_vga8x16.rs` |

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

Full 80386+ real-mode CPU implementation (partial protected mode).

| Module | Contents |
|--------|----------|
| `registers.rs` | `Registers` — EAX..EDX, ESI..EDI, EBP, ESP, EIP (32-bit), CS/DS/ES/SS/FS/GS, EFLAGS |
| `flags.rs` | Flag constants (CF, PF, AF, ZF, SF, OF, IF, DF, TF, IOPL, NT), `compute_flags_u8/16/32()`, `compute_logical_flags_*()` |
| `cpu.rs` | `X86Cpu` struct, `PrefixState` (66/67/0F/F2/F3/F0/seg overrides), instruction fetch, `Cpu` trait impl |
| `modrm.rs` | ModRM decoder with 16/32-bit addressing, SIB, segment defaults |
| `executor.rs` | `execute(opcode)` and `execute_0f(opcode)` dispatch (single-byte + 0F prefix) |
| `tracer.rs` | Instruction logging via `Tracer` trait — `FileTracer` + `NullTracer` |
| `instructions/` | 10 instruction category modules (alu/ subdir, mov/ subdir, bit, shift, control, exchange, group, stack, system) |

### `dos` — DOS Layer

Connects x86 CPU to DOS environment.

| Module | Contents |
|--------|----------|
| `system.rs` | `DosSystem` — Motherboard + X86Cpu + FileSystem + EMS + DTA + suspend + `ConsoleInputState` |
| `ivt.rs` | IVT initialization (0x00000-0x003FF), internal handler routing via CS=0xF000 |
| `interrupts/` | `dos.rs` (INT 21h), `bios.rs` (INT 10h/12h/15h/16h/1Ah/08h), `ems.rs` (INT 67h EMS 4.0) |
| `loader/` | COM loader, EXE loader (MZ header), executable detection |
| `filesystem.rs` | Multi-drive filesystem, `DiskDriver`, `DeviceKind`/`FileHandle`, wildcard matching, 8.3 formatting |
| `mcb.rs` | DOS Memory Control Block allocator (allocate/free/modify/coalesce) |
| `consts.rs` | Memory size constants (DOS_MEMORY_SIZE, MCB, MAX_MCB_SEGMENT) |

#### Key Structures in `DosSystem`

- **`dta_segment` / `dta_offset`** — configurable Disk Transfer Area address (set via INT 21h / AH=1Ah). Default: PSP segment, offset 0x0080.
- **`suspended: Option<SuspendedCall>`** — non-blocking keyboard wait. When a DOS/BIOS function needs a key but the buffer is empty, the call is suspended and the main loop polls keys instead of busy-waiting.
- **`console_input: ConsoleInputState`** — state for INT 21h / AH=0Ah (buffered input), tracks active flag, buffer address, max chars, current count.

#### `FileHandle` — Disk vs Device

File handles 0–4 are pre-registered as character devices:
- 0 = CON (stdin)
- 1 = CON (stdout)
- 2 = CON (stderr)
- 3 = AUX (COM1)
- 4 = PRN (LPT1)

Device names recognized: CON, NUL, COM1-4, LPT1-4, CLOCK$, EMMXXXX0, PRN, AUX. `resolve_device()` parses device names from DOS paths with path prefix and extension stripping. `match_pattern()` handles `*`/`?` wildcards. `format_83()` produces space-padded 8.3 names.

### `gui` — Window and Input

minifb-based implementation of the `Display` trait. Tracks previous frame's keys to detect new presses. Keyboard mapping from minifb `Key` to DOS (scancode, ASCII) pairs via `minifb_to_dos()`. Implements `pump_events()` to re-display the last cached frame via `pump()` when 50ms have elapsed (keeps window alive during keyboard-wait suspend).

### `emu_config` — Configuration Types

Config types with serde deserialization:
- `Resolution` — host window size
- `MachineConfig` — A20, EMS, DPMI, disable_change_flags
- `VideoConfig` — type + available_types
- `SoundConfig` — type + available_types
- `CpuConfig` — name, type, features
- `DriverConfig` — letter, path, read_only

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
        ├── INT 08h → bios.rs (PIT → chains to INT 1Ch)
        ├── INT 09h → EOI only
        ├── INT 10h → bios.rs (Video)
        ├── INT 12h → bios.rs (Memory size)
        ├── INT 15h → bios.rs (Extensions, A20)
        ├── INT 16h → bios.rs (Keyboard)
        ├── INT 1Ah → bios.rs (Clock)
        ├── INT 1Ch → logged
        ├── INT 20h → halt
        ├── INT 21h → dos.rs (DOS API)
        ├── INT 2Fh → dos.rs (Multiplex)
        ├── INT 67h → ems.rs (EMS 4.0)
        └── other → far call via IVT
```

### Suspend/Resume for Keyboard Input

INT 21h / AH=01h, AH=3Fh (handle=0), AH=0Ah, and INT 16h / AH=00h use `suspend_for_key()` when the keyboard buffer is empty:

1. Save the interrupt vector in `SuspendedCall` (cs/ip/flags/sp/ss are unused — re-dispatch approach)
2. In `step_frame()`, detect `suspended.is_some()`, call `pump_events()` to refresh the window, poll keyboard, and render dirty frames during wait
3. When a key arrives, call `self.dispatch(vector)` to re-execute the original INT
4. The original INT now finds a key in the buffer and completes normally

Note: The previous approach (saving/rewinding CS:IP:SP:SS:flags) has been replaced with the simpler re-dispatch mechanism.

## Main Loop (`DosSystem::step_frame`)

```
step_frame(display):
  budget = 50,000 instructions
  while budget > 0 AND not halted:
    if suspended:
      pump_events(display)
      poll display for keys
      if key available:
        re-dispatch suspended INT  // dispatch(vector)
        render dirty frame
    else:
      step()              // 1 CPU instruction + dispatch pending SWI
    tick_timer()        // every 910 ticks → check 54ms → IRQ0
    every 256 steps: poll display for keys
  if VRAM dirty: render frame, return
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

## Video System

### VideoMemory (`crate/bus/src/peripherals/video.rs`)

Supports five video modes:
- **Text80x25** — VGA 80×25 text mode with 8×16 font. Cursor tracked via `cursor_row`/`cursor_col`. Teletype output via `write_tty()` handles backspace (0x08), line feed (0x0A), carriage return (0x0D), and scrolling via `scroll_up()`. `scroll_window()` scrolls or clears rectangular regions (INT 10h AH=06h/07h).
- **Mode13h** — 320×200×8 VGA graphics mode. Framebuffer at 0xA0000.
- **Mode0Dh** — 320×200×16 planar VGA (4 bitplanes)
- **Mode0Eh** — 640×200×16 planar VGA
- **Mode10h** — 640×350×16 planar VGA
- **Mode12h** — 640×480×16 planar VGA

**VGA DAC Palette**: Full 18-bit palette (`dac: [[u8; 3]; 256]`, 6-bit per channel). Default palette has 16 CGA colors, indices 16–31 mirror 0–15, indices 32–255 approximate BIOS ramp. Ports 0x3C6 (pixel mask), 0x3C7 (read index), 0x3C8 (write index), 0x3C9 (DAC data with auto-increment).

**VGA Sequencer (0x3C4/0x3C5)**: `seq_regs[5]` with defaults `[0x01, 0x0F, 0x0F, 0x00, 0x02]`. Map mask used for planar write enable.

**VGA Graphics Controller (0x3CE/0x3CF)**: `grc_regs[9]` — supports read modes 0/1, write modes 0–3, bit mask, rotate/ALU function, set/reset, and color compare/don't care. `planar_read()` and `planar_write()` implement the full VGA planar memory access logic.

**VGA CRTC (0x3D4/0x3D5)**: `crtc_regs[0x19]` — start address (0x0C/0x0D), cursor address (0x0E/0x0F), offset (0x13), cursor scanlines (0x0A/0x0B). Methods: `start_address()`, `offset_bytes()`, `cursor_address()`, `cursor_enabled()`, `set_cursor_cell()`, `sync_cursor_from_crtc()`. BIOS cursor position is synced to CRTC cursor registers.

**VGA Attribute Controller (0x3C0/0x3C1)**: `ac_regs[0x15]` with flip-flop index/data register. Mode-dependent defaults. `ac_dac_index()` and `ac_dac_index_graphics()` map 4-bit color indices through palette + color select. `plane_enable()` (reg 0x12), `blink_enabled()` (reg 0x10 bit 3).

**VGA Input Status (port 0x3DA)**: `input_status_1()` simulates vertical/horizontal retrace timing using `retrace_epoch` (wall-clock). Resets `attr_flip_flop` on read (anti-tear).

Rendering pipeline:
1. Text mode: `VideoMemory::render_text()` — renders 640×400 using CRTC start address, line offset, cursor position/scanline, attribute controller DAC mapping, and blinking.
2. Planar modes: `VideoMemory::render_planar()` — packs 4-bit color from 4 planes into RGBA buffer using CRTC start address and offset.
3. Graphics mode 13h: `VideoMemory::render_mode13()` — 6× nearest-neighbor to 1920×1200 using CRTC start address and offset.
4. `scale_buffer()` — generic scaler to display resolution (nearest-neighbor).

### File System (`crate/dos/src/filesystem.rs`)

Multi-drive virtual filesystem with:
- **`DiskDriver`**: Maps drive letter to host directory
- **File handles**: 0–4 pre-registered as CON/CON/CON/AUX/PRN. File handles 5+ for disk files.
- **Device I/O**: CON writes to stdout, NUL discards data, PRN/AUX logs, CLOCK$/EMMXXXX0 ignored
- **`resolve_device()`**: Parses DOS device names with path prefix stripping, trailing colon support, extension stripping
- **`match_pattern()`**: Recursive wildcard matcher (`*` and `?`) for FindFirst/FindNext
- **`format_83()`**: Space-padded 8.3 filename formatting

### Motherboard (`crate/bus/src/motherboard.rs`)

Concrete `Machine` impl with:
- **A20 gate**: `apply_a20_mask()` wraps addresses when disabled. Controlled via port 0x92 or keyboard controller 0x64/0x60 command 0xD1.
- **Port dispatch**: Routes I/O to Keyboard, PIC, PIT, VideoMemory peripherals.
- **Interrupts**: `trigger_irq()` delegates to PIC.
