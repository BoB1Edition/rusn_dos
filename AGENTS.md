# AGENTS.md — карта проекта rusn_dos для ИИ-агентов

**rusn_dos** — эмулятор DOS (реальный режим x86, 16/32-бит) на Rust. Запускает .COM и MZ-.EXE программы, поддерживает INT 10h/15h/16h/1Ah/21h/2Fh/67h, текстовый режим 80x25 и VGA Mode 13h (320x200) через `minifb`.

---

## 1. Команды сборки, запуска и проверки

```bash
cargo check --workspace          # быстрая проверка типов (обязательно после правок)
cargo build                      # сборка debug-бинарника
cargo test --workspace           # юнит/интеграционные тесты (их мало: 1 в crate/x86)
cargo clippy --workspace         # линтер (в проекте не настроен строго, но желательно)

# Запуск программы в эмуляторе:
RUST_LOG=warn ./target/debug/rusn_dos run ./test/test1
RUST_LOG=warn ./target/debug/rusn_dos --graphics run ./test/test1   # с окном
./target/debug/rusn_dos --help

# Регрессионный прогон набора тестовых DOS-программ:
./test.sh                        # собирает и прогоняет всё из test/ и app/
```

- Логирование — через `env_logger` + `log` (уровень задаётся `RUST_LOG`).
- Трассировка инструкций пишется в `log`-файл, создаваемый загрузчиком (`machine.logfile`, формат `CS:IP: байты`).

---

## 2. Структура workspace

Cargo workspace: `[workspace] members = ['libs/*', 'crate/*']`, корневой пакет — бинарник `rusn_dos`.

```
rusn_dos/
├── Cargo.toml              # бинарник rusn_dos + workspace
├── config.toml             # конфиг запуска: title, resolution, [[drivers]], [[cpus]](игнорируется)
├── src/                    # ТОЧКА ВХОДА (активный код)
│   ├── main.rs             # CLI (clap): подкоманды run/win, флаги --graphics/--no_log/--config
│   └── app.rs              # App: загрузка config.toml, монтирование дисков, запуск программы
├── libs/dos_core/          # ЯДРО ЭМУЛЯТОРА (активный код) — см. раздел 3
├── crate/                  # ЭКСПЕРИМЕНТАЛЬНЫЙ рефакторинг (НЕ подключён к бинарнику!)
│   ├── bus/                #   абстракция шины: Machine/Memory/Motherboard/Peripheral + peripherals/
│   ├── x86/                #   отдельный CPU (X86Cpu), зависит от bus; единственный тест здесь
│   ├── dos/                #   пустая заготовка
│   └── gui/                #   пустая заготовка
├── test/                   # DOS-тесты: *.asm исходники и собранные бинарники (в .gitignore кроме *.asm)
├── app/                    # DOS-приложения для ручного тестирования (в .gitignore)
└── tests/                  # пусто (зарезервировано)
```

**Важно для агента:**
- Активная кодовая база: `src/` + `libs/dos_core/`. Правки функциональности вносите туда.
- `crate/*` — параллельная незавершённая архитектура. Трогайте её только если задача явно этого требует. Она не влияет на бинарник.
- `video/fonts_vga8x16.rs` (4610 строк) — данные шрифта, не редактировать вручную.

---

## 3. Карта модулей `libs/dos_core/src/`

Публичный API (`lib.rs`): `DosMachine`, `loader`, `filesystem::DiskDriver`, `video`, `ivt`, `error::Result<T>`.
Остальные модули приватные, всё взаимодействие — через `DosMachine`.

### Машина и память
| Файл | Назначение |
|---|---|
| `machine.rs` | `DosMachine` — центральный объект: память, регистры, префиксы, видео, ФС, клавиатура, EMS, A20, таймеры. Чтение/запись сегмент:офсет (`read_u8/16/32`, `write_u8/16/32`) и физическая (`read_phys_*`, `write_phys_*`). Запись/чтение в видеопамять 0xA0000–0xC0000 перехватывается и идёт в `VideoSystem`. |
| `memory.rs` | `Memory` — плоская память 16 МБ (`DOS_MEMORY_SIZE = 0xF00000` из `consts.rs`). |
| `registers.rs` | `Registers`: EAX..EDI (+8/16-битные алиасы), сегменты, IP, flags. |
| `modrm.rs` | `ModRm` — декодирование ModR/M[/SIB] и вычисление эффективного адреса. |
| `consts.rs` | Константы (размер памяти, сигнатуры MCB). |
| `mcb.rs` | Memory Control Blocks (цепочка DOS-памяти, `first_mcb_segment`). |
| `ivt.rs` | `init_ivt` — установка таблицы векторов прерываний (вектора указывают на CS=0xF000 — маркер «внутреннего» обработчика). |
| `keyboard.rs` | `Keyboard` — очередь нажатий (scancode, ascii). |

### CPU (`cpu/`)
| Файл | Назначение |
|---|---|
| `cpu/run.rs` | **Главный цикл** `run()`: чтение опкода по CS:IP → префиксы (0F/66/67/сегментные/LOCK/REP) → `executor::execute` или `execute_0f::execute_0f` → сброс префиксов. Здесь же: опрос клавиатуры окна, отрисовка видео при `video.dirty`, программный таймер (псевдо-IRQ0 → INT 08h каждые 65536 «тиков»). |
| `cpu/executor.rs` | **Диспетчер базовых опкодов** (1..700 строк match по opcode) — маршрутизирует в `cpu/execute/*` и `instructions/*`. |
| `cpu/execute/mod.rs` + `add.rs sub.rs adc.rs sbb.rs logical.rs jumps.rs checks.rs incs.rs stack.rs` | Под-диспетчеры групп опкодов (ADD/SUB/ADC/SBB/AND/OR/XOR/Jcc/CMP/INC/DEC/PUSH/POP). |
| `cpu/execute_0f.rs` | Диспетчер двухбайтовых опкодов (префикс 0x0F). |
| `cpu/flags.rs` | Флаги: константы `CF/PF/AF/ZF/SF/OF/IF/DF/TF/IOPL/NT`; `compute_flags_u8/u16/u32(current, result, cf, of, af)` и `compute_logical_flags_*`. Арифметические флаги пересчитываются через `ARITHMETIC_LOGIC_MASK`, остальные флаги сохраняются. |
| `cpu/auxiliary.rs` | `execute_rep_simple` — обёртка REP для строковых/портовых инструкций. |

### Инструкции (`instructions/`)
Вызываются из диспетчеров. Соглашение по сигнатуре — см. раздел 4.

| Файл | Назначение |
|---|---|
| `mov.rs`, `mov32.rs` | MOV, LEA, строковые (MOVS/STOS/LODS/CMPS/SCAS, 8/16/32), REP-оптимизации для видеопамяти (макросы `rep_movs_video_opt!`, `rep_stos_video_opt!`). |
| `alu/` (`arithmetic.rs group.rs logical.rs shift.rs`) | 16/8-бит арифметика, группы 80h/81h/83h/F6h/F7h/FEh, логика, MUL/DIV/IDIV, сдвиги/циклические сдвиги. |
| `alu32/` | То же для 32-бит (при префиксе 0x66). |
| `incs.rs` | INC/DEC регистровые формы 0x40–0x4F (через макросы). |
| `control.rs`, `control32.rs` | JMP/CALL/RET/LOOP/LOOPcc/Jcc/BOUND (16 и 32 бит). |
| `system.rs` | INT/IRET/IN/OUT/HLT/CLI/STI/CLC/STC/CMC/CLD/STD/SAHF/LAHF/ARPL/WAIT; **`call_interrupt(machine, vector)`** — точка входа прерываний; **`dispatch_internal_interrupt`** — маршрутизация векторов на `handle_intXX`. |
| `stack.rs` | PUSH/POP (reg, imm, mem, segment), PUSHA/POPA, PUSHAD/POPAD, PUSHF/POPF. |
| `exchange.rs` | XCHG (в т.ч. макросы `xchg_ax_reg16!`, `xchg_eax_reg32!`). |
| `segment.rs` | LES (и сегментные загрузки). |
| `extended.rs`, `extended32.rs` | MOVZX/MOVSX, доступ к CR-регистрам (заглушки), SGDT/CLTS и подобное. |
| `bcd.rs` | DAA/DAS. |

### Прерывания (`interrupts/`)
| Файл | Назначение |
|---|---|
| `bios.rs` | `handle_int10` (видео), `handle_int15` (память/A20), `handle_int16` (клавиатура), `handle_int1a` (время), `handle_int08` (таймер), `handle_int12` (объём памяти). |
| `dos.rs` | `handle_int21` (DOS API: вывод, файлы через хендлы, завершение), `handle_int2f` (мультиплекс). |
| `ems.rs` | `handle_int67` (EMS LIM 4.0; состояние в полях `ems_*` машины). |

Механика: `call_interrupt` читает вектор из IVT; если `handler_cs == 0xF000` — это внутренний обработчик (`dispatch_internal_interrupt`), иначе — реальный дальний переход со стековым фреймом FLAGS/CS/IP (см. `system.rs:704`).

### Прочее
| Файл | Назначение |
|---|---|
| `loader/mod.rs` | `load_executable(path, no_log)` — определяет тип (`detect_executable_type` по сигнатуре `MZ`=0x5A4D) и вызывает загрузчик. |
| `loader/com_loader.rs` | `ComLoader` — PSP, сегменты, стек для .COM. |
| `loader/exe_header.rs`, `loader/exe_loader.rs` | MZ-заголовок, загрузка .EXE с релокациями. |
| `filesystem.rs` | `FileSystem`, `DiskDriver` — монтирование каталогов хоста как дисков (буква, root_path, read_only), открытие/чтение/запись/закрытие, безопасное разрешение путей (защита от выхода за корень диска). |
| `video/mod.rs` | `VideoSystem` (режимы `Text80x25` / `Mode13h`), `FrameBuffer`, палитра VGA, `render_text_to_pixels`, `upscale_framebuffer`, `scale_buffer`. |
| `video/fonts_vga8x16.rs` | Битмап шрифта 8x16 (данные, не трогать). |
| `macros/mod.rs` | Макросы генерации инструкций: `inc_reg16!`, `dec_reg16!`, `mov_reg8_imm8!`, `push_reg16!`, `pop_reg16!`, `xchg_ax_reg16!`, `xchg_eax_reg32!`, **`dispatch_op32!`**, `rep_movs_video_opt!`, `rep_stos_video_opt!`. |
| `utils/logging.rs` | Пустой файл (зарезервирован). |

---

## 4. Поток выполнения (как работает эмуляция)

```
main.rs (CLI) → app.rs App::load_from_file(config.toml)
  → App::run / run_with_graphics
      → loader::load_executable()            # создаёт DosMachine, PSP, IVT, CS:IP=стартовая точка
      → init_drivers()                       # монтирование дисков из конфига
      → DosMachine::run(window)
          → cpu::run::run():  цикл пока !machine.halted:
              1. opcode = read_instr_u8(IP); IP++
              2. префиксы накапливаются в полях машины (has_operand_size_prefix и т.д.)
              3. не-префикс: execute()/execute_0f() → инструкция выполняется → префиксы сбрасываются
              4. опрос клавиатуры окна → keyboard
              5. отрисовка при video.dirty (сон 16 мс после кадра)
              6. псевдотаймер → INT 08h при включённом IF
```

---

## 5. Конвенции кода (соблюдать при правках)

1. **Сигнатура обработчика инструкции:** `fn name(machine: &mut DosMachine, prev: &[u8])`, где `prev` — уже прочитанные байты (префиксы + опкод) для логгера. Немедленные/адресные байты читаются вручную из потока:
   ```rust
   let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
   let mut bytes = prev.to_vec();
   let imm = machine.read_instr_u8(machine.registers.ip());  // затем вручную шагнуть/использовать
   bytes.push(imm);
   machine.log_instruction(csip, &bytes).ok();
   ```
2. **Выбор 16/32-бит по префиксу 0x66** — макрос `dispatch_op32!(machine, expr32, expr16)`.
3. **Адресация памятью**: `machine.read_u8/u16/u32(segment, offset)` / `write_*` (учитывает `override_segment`); физический доступ — `read_phys_*/write_phys_*` (нужен для стека через SS и внутренних структур). Стек ВСЕГДА через `ss << 4 + sp`, игнорируя сегментный переопределитель.
4. **Флаги**: не выставлять вручную побитово, использовать `flags::compute_flags_u8/u16/u32(current_flags, result, cf, of, af)` (для логики — `compute_logical_flags_*`).
5. **Первый комментарий файла**: `// Ver: N File: путь` — сохраняйте формат при изменении.
6. Комментарии в коде на русском; без необходимости новые комментарии не добавлять.
7. Неподдерживаемый опкод: `machine.print_error_exit(opcode)` (логирует и ставит `halted = true`).
8. Ошибки наружу — `Result<_, Box<dyn Error>>` (`crate::error::Result<T>`).
9. Изменения в `DosMachine` требуют обновления конструктора `new_with_memory()` (`machine.rs`).

### Как добавить новый опкод
1. Реализовать функцию в подходящем модуле `instructions/` (или `cpu/execute/*` для групп).
2. Подключить ветку в `cpu/executor.rs` (базовые), `cpu/execute_0f.rs` (0F xx) или в под-диспетчер `cpu/execute/*`.
3. Для 32-битного варианта — пара функций + `dispatch_op32!`.
4. Проверить `cargo check`, затем прогнать тест через `./test.sh` или вручную.

### Как добавить функцию прерывания
- Новый вектор: обработчик `handle_intXX` в `interrupts/*.rs` + маршрутизация в `dispatch_internal_interrupt` (`instructions/system.rs`) + вектор в `ivt.rs`.
- Новая функция INT 21h: ветка по `AH` внутри `handle_int21` (`interrupts/dos.rs`).

---

## 6. Известные проблемы (на момент ревизии)

- `cargo check` проходит; предупреждения (~40): неиспользуемые переменные (`csip`, `port`, `zf/sf/pf`, `config` в `src/main.rs`), недостижимые паттерны в `crate/x86/src/executor.rs` (дубли веток 0x8B и 0xB8..=0xBF), never-read записи в `new_flags`/`base_val`.
- Тестов почти нет: единственный — `crate/x86/tests/integration_test.rs`. Юнит-тестов у `dos_core` нет; регрессия — вручную через `test.sh`.
- `run_window` в `src/main.rs` — `todo!()`.
- `[[cpus]]` из `config.toml` читается, но не используется (`App` содержит только `title/resolution/drivers`).
- Известные баги/задачи см. `todo.todo`: LOOP/LOOPZ/LOOPNZ (0xE0–0xE2), SUB AX,imm16 (0x2D), RCL/RCR, INT 16h, корректное завершение AH=4Ch, INT 15h/E820, обработка дисков/абсолютных путей в `filesystem.rs`.
- В `machine.rs:write_phys_u16` есть отладочный `log::warn!` для адреса 0x22460 — не удалять без необходимости.

---

## 7. Чек-лист перед завершением правки

1. `cargo check --workspace` — без ошибок.
2. Не увеличивать число предупреждений.
3. Прогнать затронутый тест из `test/` (или `./test.sh` целиком) с `RUST_LOG=warn`.
4. Для новых инструкций сверять флаги/поведение с реальным x86 (8086/386).
