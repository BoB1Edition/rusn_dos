# Changelog — rusn_dos

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- **VGA CRTC (0x3D4/0x3D5)**: Full CRTC register array (`crtc_regs[0x19]`). Cursor address (0x0E/0x0F), start address (0x0C/0x0D), offset (0x13), cursor enable/scan (0x0A/0x0B) and sync between CRTC and BIOS cursor position (`sync_cursor_from_crtc()`, `set_cursor_cell()`).
- **VGA Attribute Controller (0x3C0/0x3C1)**: `ac_regs[0x15]`, `ac_index` with flip-flop toggle, mode-dependent defaults, `plane_enable()`, `ac_dac_index()`, `ac_dac_index_graphics()`, `blink_enabled()`.
- **VGA text buffer expanded**: `text_buffer` changed from `[u16; 80*25]` to `Box<[u16; 0x4000]>` supporting CRTC-panned display.
- **`render_text()`**: New native render method using CRTC start address, line offset, cursor position/scanline from CRTC registers, attribute controller DAC mapping, and blinking support.
- **`render_mode13()`**: New native render method honoring CRTC start address and offset.
- **`render_planar()`**: Now uses CRTC `start_address()` and `offset_bytes()` for correct planar framebuffer addressing.
- **Modular split of `alu.rs`**: Refactored into `crate/x86/src/instructions/alu/` subdirectory with modules: `core.rs`, `util.rs`, `eval.rs`, `inst.rs`, `bcd.rs`, `incdec.rs`, `group.rs`, and `mod.rs`.
- **Modular split of `mov.rs`**: Refactored into `crate/x86/src/instructions/mov/` subdirectory with modules: `rm.rs`, `imm.rs`, `str_ops.rs`, and `mod.rs`.
- **Visibility convention**: Established convention: private by default, `pub(crate)` when needed within crate, `pub` only when strictly required externally.

### Changed

- **`motherboard.rs`**: Added ports 0x3D4/0x3D5 (CRTC) and 0x3C0/0x3C1 (attribute controller) to I/O routing.
- **`VideoMemory::write_tty()`**: Now calls `set_cursor_cell()` to keep CRTC cursor in sync with BIOS cursor position.
- **set_cursor** (INT 10h/AH=02h): Now updates both `cursor_row`/`cursor_col` and CRTC registers via `set_cursor_cell()`.
- **cursor shape** (INT 10h/AH=01h): Now writes to CRTC registers 0x0A/0x0B instead of old `cursor_start`/`cursor_end` fields.
- **cursor read** (INT 10h/AH=03h): Returns values from CRTC registers.
- **`render_text_to_pixels()` moved to `video.rs`**: The standalone function is no longer exposed from `video.rs` — use `VideoMemory::render_text()`.
- **`upscale_framebuffer()` moved to `video.rs`**: Use `VideoMemory::render_mode13()`.
- **`DosSystem::render_frame()`**: Updated to call `video.render_text()` and `video.render_mode13()` instead of standalone functions.

### Fixed

- **VGA planar rendering start address**: Planar mode rendering now respects CRTC start address and offset, fixing off-center or misaligned output in modes 0Eh/10h/12h.
- **Text mode scroll/paging**: CRTC start address and cursor address are properly tracked.

## [Previous Releases — see git log]
