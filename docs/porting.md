# Porting Guide — Adding New Peripherals and Systems

## Adding a New Peripheral

Peripherals implement the `Peripheral` trait from `crate/bus/src/peripheral.rs`:

```rust
pub trait Peripheral {
    fn port_read(&mut self, _port: u16) -> u8 { 0 }
    fn port_write(&mut self, _port: u16, _val: u8) {}
    fn mem_read(&mut self, _addr: u32) -> u8 { 0 }
    fn mem_write(&mut self, _addr: u32, _val: u8) {}
    fn tick(&mut self) {}
}
```

### Steps

1. **Create a new file** in `crate/bus/src/peripherals/`, e.g. `mouse.rs`
2. **Add it to `crate/bus/src/peripherals/mod.rs`**
3. **Add a field** to `Motherboard` in `crate/bus/src/motherboard.rs`
4. **Initialize** in `Motherboard::new()`
5. **Route I/O ports** in `Motherboard::in_port()` / `Motherboard::out_port()`
6. **Route MMIO** in `Motherboard::read_mem_u8()` / `write_mem_u8()` if needed
7. **Call `tick()`** from the main loop (`DosSystem::step_frame`) if the peripheral needs periodic updates

### Example: Peripheral with Port I/O

```rust
// crate/bus/src/peripherals/mouse.rs
use crate::Peripheral;

pub struct Mouse {
    pub x: u16,
    pub y: u16,
    pub buttons: u8,
}

impl Mouse {
    pub fn new() -> Self {
        Self { x: 0, y: 0, buttons: 0 }
    }
}

impl Peripheral for Mouse {
    fn port_read(&mut self, port: u16) -> u8 {
        match port {
            0x23C => self.buttons,
            0x23D => self.x as u8,
            0x23E => (self.x >> 8) as u8,
            _ => 0,
        }
    }

    fn port_write(&mut self, port: u16, val: u8) {
        // handle command port
    }

    fn tick(&mut self) {
        // update internal state
    }
}
```

### Registering a BIOS/DOS Interrupt Handler

For software interrupts (INT 10h, 16h, 33h, etc.):

1. **Create handler** in `crate/dos/src/interrupts/`
   - `mouse.rs` for INT 33h
   - Or add to `bios.rs`, `dos.rs`, or `ems.rs`
2. **Add to dispatch** in `crate/dos/src/interrupts/mod.rs`:
   ```rust
   mod mouse;
   // ...
   mouse::handle_int33(sys);
   ```
3. **Set IVT entry** in `crate/dos/src/ivt.rs` (CS=0xF000 + offset)

## Adding a New Emulated System

### Structure

Each system is a separate crate in `crate/<name>/`:

```
crate/<name>/
└── src/
    └── lib.rs        — System struct, step(), load_rom()
```

### Minimum Requirements

1. **Cargo.toml** — add to workspace in root `Cargo.toml`, depend on `bus` + `log`
2. **System struct** — hold emulator state, `halted: bool`
3. **`step()`** — execute one CPU/cycle
4. **`load_rom()` / `load_cd_image()`** — load media

### Integration

- Systems that use the DOS x86 core: extend `DosSystem` or create a similar wrapper
- Standalone systems: implement `Cpu` trait from `bus::Cpu` and drive from `main.rs`
- Each system can optionally implement `Machine` for memory/peripheral access

### Example: New System Skeleton

```rust
// crate/<name>/src/lib.rs
use bus::{Cpu, Machine};

pub struct MySystem {
    pub halted: bool,
    // CPU, RAM, peripherals...
}

impl MySystem {
    pub fn new() -> Self { Self { halted: false } }
    pub fn load_rom(&mut self, data: &[u8]) {
        // load ROM into memory
    }
    pub fn step(&mut self) {
        // execute one frame/instruction
    }
}
```

## Feature Flags (Planned)

Future feature gates for conditional compilation:

```toml
[features]
default = ["full"]
full = ["audio", "video", "storage", "network", "gui"]
x86-only = []
audio = ["bus/audio"]
video = ["bus/video"]
```

Not yet implemented — currently all features are always compiled.

## Debugging

- `RUST_LOG=info` / `RUST_LOG=debug` / `RUST_LOG=trace` for logging
- `--no-log` disables opcode tracing
- Opcode tracer writes to `logopcode.txt`
- Use `X86Cpu::new_with_tracer("logopcode.txt")` for instruction-level debugging

## Testing

```bash
# Run all tests
cargo test

# Run x86 instruction tests only
cargo test -p x86

# Run with logging
RUST_LOG=debug cargo test

# Run a DOS program
cargo run Run test/simple.com
cargo run Run test/simple.exe --graphics
```
