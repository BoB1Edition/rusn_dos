# Changelog — rusn_dos

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- **SVGA / VBE (VESA BIOS Extensions)**: Support for modes 101h (640×480×256), 103h (800×600×256), 105h (1024×768×256), 112h (24-bit). LFB at 0xE0000000, bank switching via A0000 window. `vbe_info_block()`, `vbe_mode_info_block()`, `read_lfb()`, `write_lfb()`, `render_vbe()`, `set_vbe_mode()`.
- **VGA CRTC (0x3D4/0x3D5)**: Full CRTC register array (`crtc_regs[0x19]`). Cursor address (0x0E/0x0F), start address (0x0C/0x0D), offset (0x13), cursor enable/scan (0x0A/0x0B) and sync between CRTC and BIOS cursor position (`sync_cursor_from_crtc()`, `set_cursor_cell()`).
- **VGA Attribute Controller (0x3C0/0x3C1)**: `ac_regs[0x15]`, `ac_index` with flip-flop toggle, mode-dependent defaults, `plane_enable()`, `ac_dac_index()`, `ac_dac_index_graphics()`, `blink_enabled()`.
- **VGA text buffer expanded**: `text_buffer` changed from `[u16; 80*25]` to `Box<[u16; 0x4000]>` supporting CRTC-panned display.
- **`render_text()`**: New native render method using CRTC start address, line offset, cursor position/scanline from CRTC registers, attribute controller DAC mapping, and blinking support.
- **`render_mode13()`**: New native render method honoring CRTC start address and offset.
- **`render_planar()`**: Now uses CRTC `start_address()` and `offset_bytes()` for correct planar framebuffer addressing.
- **Modular split of `alu.rs`**: Refactored into `crate/x86/src/instructions/alu/` subdirectory with modules: `core.rs`, `util.rs`, `eval.rs`, `inst.rs`, `bcd.rs`, `incdec.rs`, `group.rs`, and `mod.rs`.
- **Modular split of `mov.rs`**: Refactored into `crate/x86/src/instructions/mov/` subdirectory with modules: `rm.rs`, `imm.rs`, `str_ops.rs`, and `mod.rs`.
- **Modular split of `dos.rs`**: Refactored into `crate/dos/src/interrupts/dos/` subdirectory with modules: `dispatch.rs`, `console.rs`, `file_ops.rs`, `dir.rs`, `find.rs`, `memory.rs`, `misc.rs`, and `mod.rs`.
- **Modular split of `video.rs`**: Refactored into `crate/bus/src/peripherals/video/` subdirectory with modules: `core.rs`, `dac.rs`, `planar.rs`, `render.rs`, `peripheral.rs`, `vbe.rs`, `types.rs`, and `mod.rs`.
- **Visibility convention**: Established convention: private by default, `pub(crate)` when needed within crate, `pub` only when strictly required externally.
- **Stdin Bridge**: New `src/stdin_bridge.rs` module — crossterm raw-mode polling of stdin → DOS keyboard in console mode. US scancode mapping, `RawTerminal` RAII guard.
- **CpuConfig**: Added `CpuConfig` type to `emu_config` with `name`, `r#type`, `features` fields.
- **`MachineConfig::disable_change_flags`**: New field to lock feature flags from being changed at runtime.
- **`App::parse_workdir()`**: Helper function for DOS path parsing with drive letter detection.
- **`DosSystem::program_terminated`**: New field tracking clean program exit via INT 20h/AH=4Ch.

### Changed

- **`motherboard.rs`**: Added VBE LFB (0xE0000000+) and VBE ROM (0xFE000-0xFE1FF) MMIO routing. Added ports 0x3D4/0x3D5 (CRTC) and 0x3C0/0x3C1 (attribute controller) to I/O routing. All VGA ports routed: 0x3C0-0x3C9, 0x3CE-0x3CF, 0x3D4-0x3D5, 0x3DA.
- **`VideoMemory::write_tty()`**: Now calls `set_cursor_cell()` to keep CRTC cursor in sync with BIOS cursor position.
- **set_cursor** (INT 10h/AH=02h): Now updates both `cursor_row`/`cursor_col` and CRTC registers via `set_cursor_cell()`.
- **cursor shape** (INT 10h/AH=01h): Now writes to CRTC registers 0x0A/0x0B instead of old `cursor_start`/`cursor_end` fields.
- **cursor read** (INT 10h/AH=03h): Returns values from CRTC registers.
- **`render_text_to_pixels()` moved to `video.rs`**: The standalone function is no longer exposed from `video.rs` — use `VideoMemory::render_text()`.
- **`upscale_framebuffer()` moved to `video.rs`**: Use `VideoMemory::render_mode13()`.
- **`DosSystem::render_frame()`**: Updated to call `video.render_text()` and `video.render_mode13()` instead of standalone functions.
- **CLI restructured**: `run` and `win` as clap subcommands. Removed `--log`, `--tail`, `--timeout`, `--no-log` flags. Added `--startdir`, `--workdir`, `--graphics`, `--no-stdin-keys`, `--mount`, `--max-call-depth`.
- **`DosSystem::step_frame()`**: Refactored suspend/resume loop — pump_events + keyboard polling + render during waits.
- **`App::run()`**: Uses `stdin_bridge` for keyboard input in console mode instead of direct sleeping.
- **`App::run_with_graphics()`**: Simplified loop — no `timeout`, no `--no-log`.

### Fixed

- **VGA planar rendering start address**: Planar mode rendering now respects CRTC start address and offset, fixing off-center or misaligned output in modes 0Eh/10h/12h.
- **Text mode scroll/paging**: CRTC start address and cursor address are properly tracked.

## [Previous Releases — see git log]
