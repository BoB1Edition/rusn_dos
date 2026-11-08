# DESIGN.md — Дизайн эмулятора rusn_dos

## Философия

rusn_dos спроектирован как **модульный мультисистемный эмулятор** с приоритетом IBM PC/DOS. Ключевые принципы:

1. **Разделение ответственности**: CPU (x86 crate) не знает о DOS или периферии — он работает через трейт `Machine`
2. **Модульность через трейты**: `Cpu`, `Machine`, `Peripheral`, `Display` — абстракции для замены компонентов
3. **Zero-cost абстракции**: `NullTracer` оптимизируется компилятором, `#[inline(always)]` на критических путях
4. **DOS как слой, а не ядро**: DOS обёртка (DosSystem) связывает CPU + шину + файловую систему, но не встроена в CPU
5. **Неблокирующий ввод через re-dispatch**: При suspend сохраняется только вектор; при появлении клавиши прерывание re-dispatches, а не перематывает стек
6. **Окружение DOS как конфигурация**: `EnvironmentConfig` (PATH/COMSPEC/PROMPT) живёт в `emu_config` и строится в PSP, а не хардкодится в загрузчике

## Ключевые архитектурные решения

### 1. `Machine` trait как системная шина

CPU обращается к памяти, портам и прерываниям только через `&mut dyn Machine`. Это позволяет:
- Подменять реализацию шины (Motherboard для реального железа, CpuBus для тестов)
- Добавлять MMIO, перехватывать доступ к памяти для видео/ROM
- Легко тестировать CPU изолированно

### 2. `CpuBus` — мост между CPU и DOS

```
X86Cpu::step(&mut self, machine: &mut dyn Machine)
               │
               ▼
         CpuBus { mem: &mut Motherboard }
         (реализует Machine через Motherboard)
               │
               ▼
         handle_software_interrupt → is_internal (CS=0xF000)
         Если да — pending_interrupt = Some(vector)
               │
               ▼
         DosSystem::step() забирает pending_interrupt
         и вызывает dispatch(vector)
```

### 3. Suspend/Resume для неблокирующего I/O

Проблема: INT 21h/AH=01h должен ждать нажатия клавиши, но эмулятор не может блокироваться (в графическом режиме нужно обновлять экран и опрашивать окно).

Решение: Механизм re-dispatch — `SuspendedCall` сохраняет только вектор прерывания (поля CS:IP:SP:SS:flags не используются). Основной цикл `step_frame()` при обнаружении `suspended` вызывает `pump_events()` для обновления окна, опрашивает клавиатуру и рендерит dirty-кадры. При появлении клавиши прерывание вызывается заново через `dispatch(vector)`. Во время ожидания окно обновляется каждые ~50ms.

### 4. Внутренние vs внешние прерывания

IVT вектора с CS=0xF000 считаются внутренними — их обработчики живут в Rust коде (`interrupts/mod.rs`). Вектора с CS≠0xF000 — это установленные программой обработчики, CPU прыгает на них через far call.

### 5. BAT-интерпретатор как оболочка (вариант B)

.BAT/.CMD файлы интерпретируются в Rust на уровне COMMAND.COM (`crate/dos/src/bat/runner/` — подкаталог с модулями `exec`, `commands`, `resolve`, `flow`, `types`, `util`). Каждая дочерняя программа (COM/EXE) загружается в **свежий** `DosSystem` через `loader::load_executable()`. Состояние оболочки (CWD по дискам, текущий диск, переменные окружения, errorlevel, echo) живёт в `ShellState` (Rust) и переносится в ребёнка перед стартом (`run_external()`), а после завершения читается обратно. Преимущества перед повторным использованием одного `DosSystem`: нет утечек снимков регистров/PSP/памяти между программами, изоляция ошибок ребёнка.

### 6. EXEC (INT 21h/AH=4Bh) — вложенные процессы

DOS-программа может запускать ребёнка через `exec_program()` (регистрирует `ExecRequest`) и `AH=4Dh` (`get_return_code()`). Цикл `step_frame()` обнаруживает `exec_request` и вызывает `ProcessManager::start_child()` (crate/dos/src/process.rs): снимок родителя (память, CPU, DTA, break_check, tracer) сохраняется в `parent_stack` (тип `ParentSnapshot`), ребёнок загружается in-place по пути из параметрического блока ES:BX, command tail и FCB копируются в PSP ребёнка. По `halt`/`program_terminated` вызывается `finish_child()` — восстанавливается снимок родителя, освобождается MCB ребёнка, возвращается AX=exit code/CF=0, код ребёнка доступен через `child_exit_code`/`child_exit_type`. Для трассировки ребёнку назначается отдельный tracer (`logopcode_<имя>.txt`).

### 7. `instruction_bytes` для трассировки

Каждый `fetch_u8()` не только инкрементирует EIP, но и сохраняет байт в `instruction_bytes`. Это позволяет tracer'у логировать полную последовательность байт инструкции без повторного чтения из памяти.

## Поток выполнения

```
main()
  ├── Cli::parse()                      (--max-call-depth в 1..=255, --no-stdin-keys, --graphics)
  ├── App::load_from_file(config) → config.toml (включая [environment])
  ├── resolve_program_path()            → поиск по PATH (COM/EXE/BAT), если без расширения
  │
  ├── ВЕТВЬ BAT (.BAT/.CMD):
  │     initial_dos_dir(program, workdir) → (диск, каталог) оболочки
  │     RawTerminal::try_enable()          — crossterm raw-mode (если не --no-stdin-keys)
  │     dos::bat::run_bat(...)             — интерпретатор (см. раздел BAT выше)
  │     └── Runner::run_batch(lines, labels, depth)
  │           ├── Встроенные: echo/cd/cls/goto/if/set/pause/call/exit/rem/смена диска
  │           └── Внешние: resolve_command() → run_batch (глубина+1) | run_external()
  │                 run_external():
  │                   loader::load_executable() → свежий DosSystem
  │                   перенос CWD/env/дров → child_loop(&mut ds, display, pump)
  │                   чтение CWD/env обратно в ShellState
  │
  ├── ВЕТВЬ EXE/COM:
  │     loader::load_executable(program, no_log, machine_cfg, environment, args) → инициализирует SYSVARS
  │     монтирование дисков, set_search_path_from_env(PATH), setup_program_directory()
  │     init_stdin_bridge() — RawTerminal (crossterm raw-mode, если не --no-stdin-keys)
  │     Цикл: step_frame(display)
  │           ├── step() → execute opcode
  │           ├── exec_request? → process_manager.start_child() (EXEC внутри DOS-программы)
  │           ├── halt/program_terminated + parent_stack → finish_child()
  │           ├── tick_timer() → IRQ0 каждые ~55ms
  │           ├── Опрос клавиатуры каждые 256 инструкций
  │           ├── При suspend: pump_events() + рендер dirty-кадров + re-dispatch по клавише
  │           └── Рендер при VRAM dirty (после цикла, не посередине)
  └── process::exit(код программы) — после выхода из run_main() (RAII-гарды отработали)
```

## Структуры данных

### X86Cpu
```
Registers         — EAX..EDI, EIP, CS/DS/ES/SS/FS/GS, EFLAGS
PrefixState       — has_address_size, has_operand_size, has_rep, segment_override...
gdtr/idtr         — limit + base (для LGDT/LIDT/SGDT/SIDT)
cr0               — для MOV CR0/CR3
pending_interrupt — для обработки внутренних SWI
tracer            — Box<dyn Tracer>
instruction_bytes — сбор байтов для трассировки
```

### Motherboard
```
Memory            — RAM (boxed-slice)
Keyboard          — 8042 + BIOS buffer (VecDeque<u16>)
VideoMemory       — Text80x25 / Mode13h / Mode0Dh-12h (planar), cursor (row/col), dirty flag,
                    retrace_epoch, attr_flip_flop, dac[256][3], seq_regs[5], grc_regs[9],
                    crtc_regs[0x19], ac_regs[0x15], latches[4], dac_pixel_mask,
                    dac_write/read_index
Pic               — 8259 (master + slave masks)
Pit               — 8253 (3 channels, wall-clock stub)
a20_gate          — enabled/disabled + command pending
```

### DosSystem
```
machine: Motherboard
cpu: X86Cpu
filesystem: FileSystem          — многодисковая FS + поиск по PATH (program_drive/program_dir/search_path)
ems: EmsState                   — EMS 4.0 состояние
first_mcb_segment               — начало цепочки MCB
dta_segment/offset              — Disk Transfer Area
suspended: Option<SuspendedCall> — ожидание клавиши
console_input: ConsoleInputState — AH=0Ah буферизованный ввод
exit_code: u8                   — код выхода программы
program_terminated: bool        — флаг завершения (INT 20h/AH=4Ch)
environment: EnvironmentConfig   — переменные окружения (PATH/COMSPEC/PROMPT)
break_check: bool               — INT 21h/AH=33h break check flag
exec_request: Option<ExecRequest> — запрос EXEC (путь + параметрический блок ES:BX)
process_manager: ProcessManager  — EXEC: parent_stack (Vec<ParentSnapshot>), current_psp, tracer'ы
sysvars_segment/offset          — SYSVARS (List of Lists) для INT 21h/AH=52h
child_exit_code: u8 / child_exit_type: u8 — INT 21h/AH=4Dh
```

### FileSystem
```
drivers: HashMap<char, DiskDriver>       — буква → диск
current_directories: HashMap<char, String>
open_files: HashMap<u16, FileHandle>     — хендл → Disk/Device
search_contexts: HashMap<u32, SearchContext>  — DTA → поиск
program_drive / program_dir              — каталог запущенной программы
search_path: Vec<(char, String)>         — каталоги PATH (поиск при открытии "голых" имён)
```

### ShellState (BAT-интерпретатор)
```
cwd: HashMap<char, String>  — CWD по дискам
current_drive: char
env: HashMap<String, String> — переменные окружения (рабочая копия, на наследниках через EnvironmentConfig)
echo_on: bool
errorlevel: u8
```

(`types.rs` в `bat/runner/`; вместе с `BatConfig` и `PumpFn`)

## Планируемые улучшения

1. **Плагинная периферия**: трейт Plugin с init/tick/port_read-write/mem_read-write, registry в конфиге
2. **Feature flags**: аудио, сеть, диски как опциональные модули
3. **Защищённый режим**: полноценная эмуляция сегментации, paging, DPMI
4. **EXEC AL=01h/AL=03h** (load-only/overlay) — пока возвращают CF=1/AX=1
5. **CALL :label** в BAT-интерпретаторе (подпрограммы) — на Этапе 4 не поддерживается
6. **VGA SVGA**: VESA BIOS Extensions 2.0+
7. **CPU speed control**: cycle-accurate PIT и синхронизация
8. **Отладчик**: встроенный отладчик с брейкпоинтами
