# Changelog — rusn_dos

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- **Teletype output (INT 10h / AH=0Eh)**: Full `write_tty()` implementation in video module — writes characters to VRAM with proper backspace, line feed, carriage return, and scroll. Cursor position tracked via `cursor_row`/`cursor_col`.
- **Keyboard buffer for DOS API**: INT 21h / AH=01h (read char with echo) and AH=06h (direct console I/O, DL=0xFF) now read from the keyboard buffer instead of blocking on stdin. Supports graphics mode.
- **Set DTA (INT 21h / AH=1Ah)**: `dta_segment`/`dta_offset` in `DosSystem` allow programs to set the Disk Transfer Area address. FindFirst/FindNext use `get_dta_addr()`.
- **IOCTL (INT 21h / AH=44h / AL=00h)**: Get Device Information — returns device attributes for character devices (bit 15=1) or drive number for disk files.
- **Device file system**: `DeviceKind` (CON, NUL, COM, LPT, CLOCK$, EMMXXXX0) and `FileHandle` (Disk/Device) enums. File handles 0-4 pre-registered as CON/CON/CON/AUX/PRN.
- **Device-aware file operations**: `open_file()` and `create_file()` accept device names (CON, NUL, etc.). `read_file()` handle=0 reads from keyboard. `write_file()` routes to stdout for CON, logs for PRN/AUX, discards for NUL.
- **`resolve_device()`**: Parses DOS device names with path prefix stripping and trailing colon support.
- **`match_pattern()`**: Recursive wildcard matcher for FindFirst/FindNext — handles `*` and `?` properly.
- **`format_83()` with space-padding**: 8.3 filename formatting with proper space padding (breaks some programs without it).
- **`SuspendedCall` and `suspend_for_key()`**: Non-blocking keyboard wait mechanism — when a DOS API needs a key but the buffer is empty, the call state is saved and execution is suspended. The main loop polls keys and resumes the call when a key arrives.
- **Cursor fields in VideoMemory**: `cursor_row` and `cursor_col` for text mode cursor tracking.
- **`scroll_up()` in VideoMemory**: Scrolls text mode screen up by one line when cursor reaches bottom.

### Fixed

- **INT 10h / AH=00h**: Restored video mode set functionality (was corrupted by copy-paste into keyboard handler).
- **INT 10h / AH=0Eh**: Now writes to video RAM instead of only printing to host stdout. Text mode output works correctly in emulated video.
- **INT 21h / AH=01h**: No longer blocks emulator on stdin. Uses keyboard buffer with suspend/resume.
- **INT 21h / AH=06h (DL=0xFF)**: Now checks keyboard buffer instead of always returning "no key".
- **INT 21h / AH=3Fh (handle=0)**: Reads from keyboard buffer for stdin (CON) instead of returning zero bytes.
- **INT 2Fh / AX=1687h**: Proper DPMI host check returning CF=1 (Not Installed) instead of falling through to generic handler.
- **FindFirst/FindNext DTA**: Uses configurable DTA address (via AH=1Ah) instead of hardcoded DS:0x0080.
- **8.3 filename formatting**: Space-padded in DTA structure, fixing compatibility with programs that expect padding.
- **Video mode set**: Now correctly writes to CRTC-like ports when set via motherboard dispatch.

### Changed

- **`filesystem.rs`**: Major refactor — added device abstraction layer. File handles now distinguish between disk files and character devices.
- **`dos.rs`**: INT 21h dispatch updated: AH=1Ah and AH=44h added. INT 2Fh uses full AX match for 1687h. `get_dta_addr()` delegates to `DosSystem`.
- **`system.rs`**: Added DTA fields, suspended-call mechanism, `get_dta_addr()` method.
- **`video.rs`**: Added cursor tracking, scroll, write_tty. Version bumped to 2.

### Documentation

- Added `docs/architecture.md` — system architecture and crate layout
- Added `docs/x86.md` — complete instruction reference
- Added `docs/porting.md` — guide for adding peripherals and systems
- Updated `TODO.md` — synced with current implementation status
