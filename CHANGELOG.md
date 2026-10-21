# Changelog — rusn_dos

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- **VGA Input Status 1 (port 0x3DA)**: `input_status_1()` simulates vertical/horizontal retrace timing (60 Hz frame, ~1.4ms VBLANK, ~11µs HBLANK) using `retrace_epoch`. Anti-tear register — resets `attr_flip_flop` on read.
- **VGA Attribute Controller flip-flop**: `attr_flip_flop` field in VideoMemory toggles for VGA attribute sequencing.
- **`pump_events()` on `Display` trait**: Default no-op method; `EmuWindow` implements it to call `window.update_with_buffer` for periodic frame refreshes during suspend.
- **`tty_write()` in bios.rs**: Unified teletype output function — writes character to video RAM via `write_tty()` AND flushes to host stdout. Replaces raw `print!`/`println!` in DOS console functions.
- **Frame caching in EmuWindow**: `last_frame`/`last_size`/`last_update` keep the last rendered frame. `pump()` re-displays it on timer (every 50ms) to keep the window alive during keyboard-wait suspend.
- **`step_frame()` — suspend re-dispatch**: When a key arrives during suspend, the original INT vector is re-dispatched instead of merely clearing the suspended flag. This allows buffered input and blocking reads to resume correctly.
- **`step_frame()` — render at end of budget**: Dirty-check rendering moved after the instruction budget loop (instead of returning mid-budget) so the frame is painted even when no VRAM write happens in the last batch.
- **Comprehensive test suite**: `test/testvga.asm` — 20-test COM file covering DOS version, memory, drive, directory, teletype, backspace, direct B800h write, port 3DAh, video mode, A20, DPMI, FindFirst, CON device, IOCTL, keyboard polling, suspend/resume, buffered input, Mode 13h graphics, and CON read.

### Fixed

- **INT 21h / AH=02h, AH=06h, AH=09h, AH=0Ah, AH=3Fh**: All console output functions now route through `tty_write()` — writes to both video RAM and host stdout. Previously used raw `print!()` which bypassed VRAM.
- **INT 21h / AH=0Ah (buffered input) — Backspace echo**: Now uses `tty_write()` for backspace (0x08 + space + 0x08) so video cursor moves correctly.
- **VGA port read — added 0x3DA routing**: `Motherboard::in_port()` now dispatches port 0x3DA to `video.port_read()`, enabling programs to poll vertical retrace.
- **`set_mode()` cursor reset**: `cursor_row`/`cursor_col` are now zeroed on mode set.
- **`input_status_1()` resets `attr_flip_flop`**: Correct VGA behavior — reading 0x3DA resets the attribute controller flip-flop.
- **`read_mmio`/`write_mmio` TEXT_BASE check**: Text buffer is at B8000 (offset 0x18000), not at 0. Added bounds check to prevent reading/writing outside the text region.
- **`suspend_for_key()` simplified**: Removed complex stack-unwind logic (commented-out code). Now only stores the vector; fields cs/ip/flags/sp/ss set to 0 as unused.

### Changed

- **`step_frame()` — restructured**: Suspended path now calls `pump_events()` on display, renders dirty frames during wait, re-dispatches original INT on key press. Render moved to end of budget loop.
- **`peripheral.rs` trait docs stripped**: Removed verbose doc comments. Trait methods now have no doc comments (follows project convention of minimal comments).
- **`EmuWindow::poll_keys()` auto-pumps**: If 50ms have passed since last frame update, calls `pump()` to keep the window responsive.
- **`gui/src/lib.rs` — major refactor**: Added frame caching (last_frame, last_size, last_update). `update()` saves frame and renders immediately. New `pump_events()` implementation.
- **DOS console functions**: All `print!()`/`println!()` replaced with `tty_write()` for video + stdout dual output.

### Documentation

- Updated `test/testvga.asm` — comprehensive 20-test suite

## [Previous Releases — see git log]
