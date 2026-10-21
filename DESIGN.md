# DESIGN.md — Дизайн эмулятора rusn_dos

## Философия

rusn_dos спроектирован как **модульный мультисистемный эмулятор** с приоритетом IBM PC/DOS. Ключевые принципы:

1. **Разделение ответственности**: CPU (x86 crate) не знает о DOS или периферии — он работает через трейт `Machine`
2. **Модульность через трейты**: `Cpu`, `Machine`, `Peripheral`, `Display` — абстракции для замены компонентов
3. **Zero-cost абстракции**: `NullTracer` оптимизируется компилятором, `#[inline(always)]` на критических путях
4. **DOS как слой, а не ядро**: DOS обёртка (DosSystem) связывает CPU + шину + файловую систему, но не встроена в CPU
5. **Неблокирующий ввод через re-dispatch**: При suspend сохраняется только вектор; при появлении клавиши прерывание re-dispatches, а не перематывает стек

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

### 5. `instruction_bytes` для трассировки

Каждый `fetch_u8()` не только инкрементирует EIP, но и сохраняет байт в `instruction_bytes`. Это позволяет tracer'у логировать полную последовательность байт инструкции без повторного чтения из памяти.

## Поток выполнения

```
main()
  ├── Cli::parse()
  ├── App::load_from_file(config) → config.toml
  ├── loader::load_executable() → DosSystem
  │     ├── Motherboard::new(DOS_MEMORY_SIZE)
  │     ├── init_ivt() — IVT at 0x0000
  │     ├── Загрузка COM/EXE в память
  │     ├── Настройка MCB
  │     └── Инициализация регистров
  ├── init_drivers() — монтирование дисков
  ├── setup_program_directory()
  └── Цикл: step_frame(display)
        ├── step() → execute opcode
         ├── tick_timer() → IRQ0 каждые ~55ms
         ├── Опрос клавиатуры каждые 256 инструкций
         ├── При suspend: pump_events() + рендер dirty-кадров + re-dispatch по клавише
         └── Рендер при VRAM dirty (после цикла, не посередине)
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
VideoMemory       — Text80x25 / Mode13h, cursor (row/col), dirty flag, retrace_epoch, attr_flip_flop
Pic               — 8259 (master + slave masks)
Pit               — 8253 (3 channels, wall-clock stub)
a20_gate          — enabled/disabled + command pending
```

### DosSystem
```
machine: Motherboard
cpu: X86Cpu
filesystem: FileSystem     — многодисковая FS
ems: EmsState              — EMS 4.0 состояние
first_mcb_segment          — начало цепочки MCB
dta_segment/offset         — Disk Transfer Area
suspended: Option<SuspendedCall>  — ожидание клавиши
console_input: ConsoleInputState   — AH=0Ah буферизованный ввод
```

### FileSystem
```
drivers: HashMap<char, DiskDriver>       — буква → диск
current_directories: HashMap<char, String>
open_files: HashMap<u16, FileHandle>     — хендл → Disk/Device
search_contexts: HashMap<u32, SearchContext>  — DTA → поиск
```

## Планируемые улучшения

1. **Плагинная периферия**: трейт Plugin с init/tick/port_read-write/mem_read-write, registry в конфиге
2. **Feature flags**: аудио, сеть, диски как опциональные модули
3. **Защищённый режим**: полноценная эмуляция сегментации, paging, DPMI
4. **INT 4B00h (EXEC)**: загрузка дочерних процессов
5. **VGA CRTC/DAC/SVGA**: полная эмуляция VGA-регистров
6. **CPU speed control**: cycle-accurate PIT и синхронизация
7. **Отладчик**: встроенный отладчик с брейкпоинтами
