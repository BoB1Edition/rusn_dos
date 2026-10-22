# Changelog — rusn_dos

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- **VGA planar modes (0Dh/0Eh/10h/12h)**: `VideoMode` extended with `Mode0Dh` (320×200×16), `Mode0Eh` (640×200×16), `Mode10h` (640×350×16), `Mode12h` (640×480×16). `set_mode()` clears 4 planes on planar switch. `set_mode_u8()` handles modes 0x0D-0x12 with bit 7 mask.
- **DAC palette (18-bit, 256 colors)**: `dac: [[u8; 3]; 256]` field with 6-bit per channel. `default_dac_palette()` — 16 standard CGA colors (6-bit), indices 16-31 mirror 0-15, 32–255 BIOS ramp. `dac_pixel_mask` (0x3C6), `dac_write_index` (0x3C8), `dac_read_index` (0x3C7), `dac_cycle`, `dac_mode_read` — full read/write via port 0x3C9 with auto-increment.
- **VGA Sequencer registers**: `seq_index`/`seq_regs[5]` (ports 0x3C4/0x3C5). Defaults: `[0x01, 0x0F, 0x0F, 0x00, 0x02]`. Map mask (`seq_regs[2]`) used in `planar_write()`.
- **VGA Graphics Controller registers**: `grc_index`/`grc_regs[9]` (ports 0x3CE/0x3CF). Supports read modes 0/1, write modes 0–3, bit mask, rotate, ALU function, set/reset, color compare, color don't care.
- **Planar read/write logic**: `planar_read(offset)` loads latches from 4 planes, returns plane selected by read map. `planar_write(offset, value)` implements write modes 0-3 with map mask, bit mask, rotate, ALU ops, set/reset, and color don't care.
- **`render_planar(w, h, palette)`**: Packs 4-bit color indices from 4 planes into RGBA pixel buffer for planar mode rendering.
- **`scroll_window(top, left, bottom, right, lines, attr, up)`**: Scrolls/clears rectangular region of text buffer. Used by INT 10h AH=06h/07h.
- **`write_cell_repeat(ch, attr, count)`**: Writes char+attr repeatedly from cursor position. Used by INT 10h AH=09h/0Ah.
- **INT 10h functions (AH=01h–0Fh, 12h)**: AH=01h (cursor shape), AH=02h (set cursor position, BDA sync), AH=03h (read cursor), AH=05h (select page), AH=06h/07h (scroll up/down), AH=08h (read char+attr), AH=09h/0Ah (write char+attr), AH=0Fh (mode number), AH=12h (alternate select — stub). New helpers: `sync_cursor_bda()`, `scroll_window_int()`, `mode_number()`, `tty_write()`.
- **INT 21h/AH=07h**: Direct character input, no echo (blocking, no Ctrl+C check).
- **INT 21h/AH=08h**: Character input without echo (blocking, with Ctrl+C → INT 23h check).
- **VGA DAC port I/O routing**: Ports 0x3C6–0x3C9, 0x3DA added to Motherboard `in_port()`/`out_port()` for video peripheral.
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
