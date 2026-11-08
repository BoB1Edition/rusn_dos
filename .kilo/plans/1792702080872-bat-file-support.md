# Plan: .BAT File Support for DOS Emulator

## Goal

Add support for executing DOS batch files (.BAT) from the CLI. The interpreter
runs in Rust, parses `.BAT` files, and executes both built-in commands (echo,
cls, goto, if errorlevel, rem) and external programs (EXE/COM) using the
existing loader infrastructure. One `DosSystem` is reused across all commands
in the batch.

---

## User workflow

```toml
# config.toml
[[drivers]]
letter = "C"
path = "app/"
read_only = false
```

```bash
cargo run Run app/XCOM/UFO.BAT --workdir ./
```

The interpreter mounts the drives, changes CWD to `\XCOM` (parent of the BAT
file relative to the mount root), then executes each command.

---

## New module: `crate/dos/src/batch.rs`

### Data structures

```rust
pub struct BatchInterpreter {
    commands: Vec<BatchCommand>,   // pre-parsed lines
    labels: HashMap<String, usize>, // label name → command index
    pc: usize,                     // current command index
    errorlevel: u8,
    echo_on: bool,                 // from `echo on` / `echo off`
    running: bool,
}

pub enum BatchCommand {
    Builtin(Builtin),
    Program { path: String, args: String },
    Label(String),                 // not executed, just a marker
}

pub enum Builtin {
    Echo(String),
    EchoOff,
    EchoOn,
    Cls,
    Rem,
    Goto(String),
    IfErrorLevel { n: u8, target: String },
}
```

### Parsing (`BatchInterpreter::new`)
- Open the `.BAT` file via `std::fs::read_to_string`
- Split on `\n`, trim `\r`
- Pass 1: strip `@` prefix, identify labels (`:label`), build `labels` map
- Pass 2: parse each non-label, non-empty line into `BatchCommand`
- Store `commands` Vec

### Execution (`BatchInterpreter::next`)
- Return the current command and advance `pc`
- `Label` entries are skipped transparently

---

## Integration: `src/app.rs`

### Route `.bat` files
In `run()` and `run_with_graphics()`, detect `.bat`/`.BAT` extension and call
`run_batch()` instead of `run()`.

### `run_batch(program, no_log, graphics)` flow
1. Create `DosSystem` via `loader::load_executable()` — the same initial call
   (loader treats `.BAT` as a COM stub that halts immediately, or we add
   `loader::create_empty_system()` that does motherboard + IVT + MCB + filesystem
   init without loading any program)
2. Mount drivers via `init_drivers()`
3. Set CWD to the BAT's parent directory via `setup_program_directory()`
4. Create `BatchInterpreter::new(&bat_path, &filesystem)`
5. Loop:
   a. Call `interpreter.next_command()`
   b. If `None` → break
   c. If `Builtin` → execute in Rust
   d. If `Program` → call `loader::reload_executable(&mut ds, &path, args)`,
      then `step_frame()` until halt, capture exit code,
      call `interpreter.set_errorlevel(exit_code)`
6. Log "batch finished"

---

## Reload mechanism: `crate/dos/src/loader/mod.rs`

### New function: `reload_executable(ds, program_path, cmd_tail, no_log)`

This reuses an existing `DosSystem` to load and run a new program:

1. **Re-init IVT** — `crate::ivt::init_ivt(&mut ds.machine)`
2. **Reset MCB chain** — `mcb::init_memory_map(&mut ds.machine, ds.first_mcb_segment)`
3. **Load program** — write PSP + code into the same region
   - COM: `PSP_SEGMENT=0x1000`, code at `CS:0100`, stack at `SS:FFFE`
   - EXE: `LOAD_SEGMENT=0x1000`, PSP at `LOAD_SEGMENT-0x10`
   - Use same code as `exe_loader::exec` / `com_loader::exec` but writing to
     `ds.machine` and `ds.cpu` instead of creating new ones
4. **Set up PSP command tail** (offset 0x80) — write length byte + ASCII tail
   (strip quotes from quoted args per DOS convention)
5. **Reset CPU**:
   - Registers (CS:IP, SS:SP, DS, ES, FS, GS)
   - `ds.cpu.halted = false`
   - Clear `ds.suspended`
   - Reset `ds.console_input`
6. **Close stale file handles** in `ds.filesystem` (handles 5+)
7. **Preserve**:
   - `ds.filesystem` drive map + current directories
   - `ds.machine.video` — VRAM persists between programs
   - `ds.machine.keyboard` — buffered keystrokes available to next program

### Refactor loaders

Add public methods `ComLoader::reload_into(ds, data)` and
`ExeLoader::reload_into(ds, data)` that copy the program-loading logic from
`exec()` but operate on existing `&mut DosSystem` instead of creating a new one.

---

## Built-in command implementations

All built-ins execute in Rust against `&mut DosSystem`:

### `echo text`
Call `tty_write()` for each byte followed by `\r\n`.

### `echo off` / `echo on`
Set `interpreter.echo_on`. (Not used for internal interpreter output, but
semantically correct for any future command-echo feature.)

### `cls`
Set video mode to `Text80x25` (INT 10h AH=00h AL=03h equivalent), clear text
buffer:
```rust
ds.machine.video.set_mode(VideoMode::Text80x25);
```

### `rem` / `::`
Do nothing.

### `goto label`
Look up label in `interpreter.labels`, if found set `interpreter.pc` to index,
if not found log warning and stop.

### `if errorlevel N command`
Compare `interpreter.errorlevel >= n`. If true, execute the rest of the line
(after `if errorlevel N`). The rest is parsed as a normal command (goto, program,
echo, etc.).

### `@` prefix
Stripped during parsing — the rest of the line is processed normally.

---

## Files to modify

| File | Action |
|------|--------|
| `crate/dos/src/batch.rs` | **CREATE** — interpreter |
| `crate/dos/src/lib.rs` | Add `pub mod batch;` |
| `src/app.rs` | Add `run_batch()`, route `.bat` in `run()`/`run_with_graphics()` |
| `crate/dos/src/loader/mod.rs` | Add `reload_executable()`, add `create_empty_system()` |
| `crate/dos/src/loader/com_loader.rs` | Add `ComLoader::reload_into()` |
| `crate/dos/src/loader/exe_loader.rs` | Add `ExeLoader::reload_into()` |

---

## Error handling
- Malformed BAT file → log warning, skip line, continue
- Program not found via `filesystem.resolve_path()` → log error, set errorlevel, continue
- Goto to nonexistent label → log warning, stop batch
- Infinite goto loop prevention: if we see >1000 commands executed, stop

---

## Validation

1. `cargo build` — must compile with no new warnings
2. `cargo clippy -p dos` — no clippy warnings
3. Hand-crafted test BAT:
   ```
   @echo off
   cls
   echo Hello from BAT
   test\simple.com
   if errorlevel 1 goto fail
   goto end
   :fail
   echo FAILED
   :end
   echo All done
   ```
4. Run against `app/XCOM/UFO.BAT` with `--graphics` — geoscape + tactical
   should load and run correctly
5. Verify errorlevel propagation: `test/simple.com` returns a known exit code,
   `if errorlevel` checks it correctly
6. Verify goto both forward (`goto end`) and backward (`goto begin`)
7. Verify `@echo off` doesn't produce unwanted output
8. Verify `::` and `rem` lines are silently skipped
