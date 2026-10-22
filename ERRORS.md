# ERRORS.md — Описание текущих ошибок в реализации

## Категории ошибок

- **[CRITICAL]** — блокирует работу программ
- **[BUG]** — неверное поведение
- **[STUB]** — код-заглушка, требующий реализации
- **[DEAD]** — мёртвый код
- **[CLEANUP]** — избыточный дублирующийся код, можно заменить вызовом общей функции

---

## 1. Регистры (crate/x86/src/registers.rs)

### 1.1. [BUG] `set_eflags()` не форсирует бит 1
- **Файл:** `registers.rs:324-326`
- **Проблема:** `set_flags()` на строке 318 устанавливает `| 0x2` (бит 1 всегда 1 на 8086). `set_eflags()` на строке 324 этого не делает, что позволяет сбросить зарезервированный бит.
- **Fix:** добавить `| 0x2` в `set_eflags()`.
- `0x2` — зарезервированный бит 1, должен быть всегда 1.

### 1.2. [CLEANUP] `set_ss()` — debug log
- **Файл:** `registers.rs:313`
- `log::debug!("set sp ...")` — остаточный отладочный вывод, засоряет лог.

---

## 2. ModRM (crate/x86/src/modrm.rs)

### 2.1. [BUG] SIB: base_field==5 + mod_field!=0 — ss_default установлен слишком рано
- **Файл:** `modrm.rs:94`
- **Проблема:** `ss_default = base_field == 4 || base_field == 5` устанавливается до проверки `base_field == 5 && mod_field == 0` (return early на строке 113). Для `base_field == 5 && mod_field != 0` всё корректно. Но `ss_default` также применяется через `ss_default || (rm_field == 5 && mod != 0)` на строке 157, что приводит к двойной проверке. Не ошибка, но избыточность.
- Исправить: убрать `ss_default` в SIB-path для base_field==5, т.к. строка 157 перепроверяет.

---

## 3. ALU (crate/x86/src/instructions/alu.rs)

### 3.1. [STUB] AF не вычисляется в shift/rotate (передаётся false)
- **Файл:** `shift.rs:91,112,131`
- **Проблема:** `compute_flags_u8/16/32()` получают `af = false`. На реальном x86 все арифметические и сдвиговые инструкции могут менять AF (кроме логических — AND/OR/XOR/TEST — AF undefined).
- **Проявление:** Программы, проверяющие AF после сдвига, получают неверное значение.
- **Fix:** Добавить вычисление AF для сдвигов/вращений.

### 3.2. [BUG] DAA/DAS — OF не меняется, но читается из старого состояния
- **Файл:** `alu.rs:799,824`
- `let of = flags::test_of(cpu.registers.flags());` — OF undefined для DAA/DAS на реальном x86. Код сохраняет старое значение, что приемлемо на практике.

---

## 4. Строковые инструкции (crate/x86/src/instructions/mov.rs)

### 4.1. [BUG] INS/OUTS с 0x66 — нестандартная обработка
- **Файл:** `mov.rs:595-662`
- **Проблема:** `insw()` и `outsw()` при `has_operand_size == true` читают/пишут 4 последовательных порта вместо того, чтобы эмулировать 32-битный INSD/OUTSD из одного порта.
- **Fix:** Для 16-битного режима с 0x66 ожидается 32-битный IN/OUT (читается/пишется 4 байта за один порт, а не 4 отдельных IN).

---

## 5. Системные инструкции (crate/x86/src/instructions/system.rs)

### 5.1. [BUG] SGDT/SIDT — пишут заглушку
- **Файл:** `system.rs:319-323`
- **Проблема:** `machine.write_mem_u16(addr, 0xFFFF)` и `write_mem_u32(addr+2, 0)`. Не пишут реальные GDTR/IDTR.
- **Fix:** Заменить на запись `cpu.gdtr_limit`, `cpu.gdtr_base` (скорректированных на 24-bit).

### 5.2. [BUG] `cbw` при `has_operand_size` — неправильное имя
- **Файл:** `system.rs:87-95`
- **Проблема:** С префиксом 0x66 инструкция 0x98 это `CWDE` (CW → CDQE в 64-bit, но для 32-bit это CWDE: sign-extend AX to EAX), а не CBW. Реализация делает sign-extend AX в EAX — правильно для CWDE.
- Имя функции `cbw()` вводит в заблуждение, хотя поведение верное.
- **Fix:** Переименовать в `cwde_or_cbw()` или разделить.

### 5.3. [BUG] `cwd()` при `has_operand_size` — неправильное имя (должно быть CDQ)
- **Файл:** `system.rs:97-105`
- С 0x66 опкод 0x99 = CDQ (sign-extend EAX to EDX:EAX). Реализация делает sign-extend EAX → EDX — верно.

### 5.4. [STUB] RDTSC — возвращает 0
- **Файл:** `system.rs:516-519`
- **Проблема:** `cpu.registers.set_eax(0); set_edx(0)`. Нет эмуляции счетчика тактов.
- **Fix:** Использовать `std::time::Instant` или счётчик инструкций CPU.

### 5.5. [BUG] INT1 (0xF1) — halt вместо прерывания
- **Файл:** `system.rs:442-449`, `executor.rs:317`
- **Проблема:** 0xF1 (INT1/ICEBP) должен вызывать `software_interrupt(cpu, machine, 1)`, а не холдить CPU.

### 5.6. [BUG] IN/OUT AX — с 0x66 читает/пишет 4 отдельных порта
- **Файл:** `system.rs:151-176`
- **Проблема:** `read_port_ax()`/`write_port_ax()` при `has_operand_size` делает 4× `in_port` на последовательных портах. Правильное поведение: INSD/OUTSD читает/пишет 32-битное значение из одного порта.
- Но на практике у многих устройств порты идут подряд, так что может работать. **Поведение не соответствует спецификации.**

---

## 6. Сдвиги/вращения (crate/x86/src/instructions/shift.rs)

### 6.1. [BUG] OF для ROL/ROR/RCL/RCR при count > 1
- **Файл:** `shift.rs:140,150,169,189`
- **Проблема:** На реальном x86 OF вычисляется только для count=1. Для count>1 OF undefined. Код вычисляет OF только при `count == 1` — **верно**.

### 6.2. [BUG] SAR — OF всегда false
- **Файл:** `shift.rs:239`
- На реальном x86 SAR с count=1 устанавливает OF в 0 всегда. Код делает `let of = false` — **правильно для count=1**, для count>1 OF undefined.

### 6.3. [CLEANUP] AF в compute_flags не передаётся (hardcoded false)
- **Файл:** `shift.rs:91,112,131`
- AF не вычисляется для сдвигов/вращений. На реальном x86 AF undefined для всех сдвигов, так что это приемлемо. **Но для RCL/RCR AF = undefined, не false.**

---

## 7. Групповые инструкции (crate/x86/src/instructions/group.rs)

### 7.1. [CLEANUP] `mul_flags()` — дублирует `set_mul_overflow()` в system.rs
- **Файл:** `group.rs:381-389` и `system.rs:242-250`
- **Проблема:** Две одинаковые функции, устанавливающие CF/OF для умножения.
- **Fix:** Вынести в общий модуль или использовать одну.

### 7.2. [STUB] CALL far через Grp FF /3 — не поддерживает register mode
- **Файл:** `group.rs:333-347`
- `if modrm.is_register_mode() { cpu.halted = true; }` — FF /3 с mod=3 это FAR CALL через регистр (невалидно на x86, но halt — не лучшее решение).

---

## 8. Битовые инструкции (crate/x86/src/instructions/bit.rs)

### 8.1. [BUG] BT/BTS/BTR/BTC для памяти — неверная адресация бита
- **Файл:** `bit.rs:207-212`
- **Проблема:** Для memory operand, если индекс бита >= размера операнда, x86 адресует через (offset + index/8) и (bit = index % 8). Текущий код использует `(value >> (index & 0xF)) & 1` — работает только когда index < 16. Для index >= 16 доступ должен перейти на следующий байт в памяти.

### 8.2. [CLEANUP] `set_cf()` — дублирует системную установку CF
- **Файл:** `bit.rs:430-438`
- **Проблема:** В файле есть своя `set_cf()`, хотя во всём проекте CF устанавливается через `flags` напрямую. Можно заменить на общую функцию.

### 8.3. [MISSING] BSF (0F BC) и BSR (0F BD) — не реализованы
- **Файл:** `executor.rs:368-404`
- В `execute_0f()` нет обработки опкодов 0xBC (BSF) и 0xBD (BSR).

---

## 9. Исполнитель (crate/x86/src/executor.rs)

### 9.1. [BUG] Отсутствуют опкоды 0x37 (AAA) и 0x3F (AAS)
- **Файл:** `executor.rs:6-366`
- **Проблема:** AAA и AAS не диспатчатся, ведут к `halt_unknown`. Это базовые BCD-инструкции, нужны для некоторых DOS-программ.

### 9.2. [MISSING] 0x0F 0xBC (BSF) и 0x0F 0xBD (BSR)
- **Файл:** `executor.rs:368-404`
- Бит-сканирующие инструкции, нужны для Windows-совместимости и некоторых защищённых программ.

### 9.3. [MISSING] 0x0F 0x40-0x4F (CMOVcc) — нет
- Условные MOV для Pentium Pro+. Не критично для DOS, но нужно для Win95.

---

## 10. Шина/Материнская плата (crate/bus/)

### 10.1. [STUB] PIT (8253) — порты возвращают wall-clock, нет счётной эмуляции
- **Файл:** `peripherals/pit.rs`
- `port_read(0x40-0x42)` возвращает `(SystemTime::now().duration_since(UNIX_EPOCH).as_millis() & 0xFF) as u8`. Не эмулирует реальный счётчик PIT.
- `tick()` всегда возвращает true (всегда IRQ), но **никогда не вызывается из Motherboard**.

### 10.2. [BUG] PIC — не используется для IRQ
- **Файл:** `peripherals/pic.rs`
- `get_next_interrupt()` не вызывается. Таймер ставит IRQ0 напрямую через `DosSystem::raise_interrupt()`, минуя PIC.

### 10.3. [FIXED] VGA — sequencer, graphics controller, DAC реализованы
- **Статус:** Sequencer (0x3C4/3C5), graphics controller (0x3CE/3CF), DAC (0x3C6-0x3C9) полностью реализованы в commit 927706e
- CRTC (0x3D4/0x3D5) пока не обрабатывается

### 10.4. [BUG] A20 mask — теряет верхние биты
- **Файл:** `motherboard.rs:34-40`
- `addr & 0xFFFFF` корректно для real-mode wrapping, но для protected mode адреса выше 16MB обрезаются до 1MB region.

### 10.5. [BUG] IRQ1 (клавиатура) — не генерируется аппаратно
- **Файл:** `peripherals/keyboard.rs`, `interrupts/mod.rs:15-18`
- INT 09h отправляет EOI, но не читает scancode. Клавиши поступают через буфер в `DosSystem::step_frame()`, минуя цепочку IRQ1 → INT 09h.

---

## 11. DOS (crate/dos/)

### 11.1. [BUG] INT 10h / AH=0Eh — не пишет в видеопамять в Mode 13h
- **Файл:** `interrupts/bios.rs`
- В режиме 13h символы не выводятся на экран, только `log::debug`. Замечание: существует `tty_write()` который пишет и в VRAM и в stdout, но он используется только консольными функциями DOS (INT 21h), а не напрямую INT 10h.

### 11.2. [FIXED] INT 10h / AH=02h — установка курсора
- Исправлено: `cursor_row`/`cursor_col` сохраняются, синхронизируются с BDA
- **Статус:** Исправлено в commit 927706e

### 11.3. [BUG] INT 16h / AH=00h — использует suspend/resume, но может подвиснуть
- **Файл:** `interrupts/bios.rs:63-68`
- Использует `suspend_for_key()`, что корректно. Убедиться, что всегда есть wakeup.

### 11.4. [BUG] EMS — страницы не маппятся реально
- **Файл:** `interrupts/ems.rs:17-24`
- `AH=42h` проверяет handle, но не копирует данные в page frame (0xD000). Любая программа, использующая EMS real pages, получит мусор.

### 11.5. [BUG] `run_window()` — todo!()
- **Файл:** `main.rs:78-80`
- Команда `Win` не реализована.

### 11.6. [STUB] INT 21h / AH=00h — terminate, но не освобождает память
- **Файл:** `interrupts/dos.rs:14-20`
- Просто ставит `halted = true`. Не освобождает выделенные блоки MCB, не закрывает файлы.

### 11.7. [MISSING] INT 21h / AH=4B00h — EXEC (загрузка дочерних процессов)
- **Файл:** `interrupts/dos.rs`
- Критично для COMMAND.COM и многих программ.

### 11.8. [MISSING] INT 21h / AH=2Ah (Get System Date), AH=2Ch (Get System Time)
- Необходимы для программ, показывающих дату/время.

---

## 12. Прочее

### 12.1. [CLEANUP] `set_carry_flag()` в dos.rs дублирует set_cf из bit.rs
- **Файл:** `interrupts/dos.rs:193-201`
- Локальная функция. Есть аналоги в `bit.rs:430-438` и `alu.rs` (косвенно).

### 12.2. [CLEANUP] `cpu.prefixes.has_operand_size` назван confusingly
- В реальном x86 0x66 = операнд-размер override. В 16-bit режиме он переключает на 32-bit; в 32-bit режиме переключает на 16-bit. Сейчас эмулятор **не поддерживает 32-bit default**, поэтому `has_operand_size` = "32-bit mode". При добавлении защищённого режима это сломается.

### 12.3. [CLEANUP] `Pending_interrupt` — только один слот
- `cpu.rs:31` — `pending_interrupt: Option<u8>`. Если DOS-обработчик прерывания вызывает `software_interrupt()`, предыдущий pending перезаписывается.

### 12.4. [BUG] IVT — не все стандартные вектора установлены
- **Файл:** `ivt.rs`
- Отсутствуют вектора: INT 00h (Division by zero), INT 01h (Single Step), INT 02h (NMI), INT 03h (Breakpoint), INT 04h (INTO), INT 05h (BOUND), INT 06h (Invalid Opcode), INT 07h (Coprocessor), INT 11h (Equipment Check), INT 14h (Serial), INT 17h (Printer), INT 18h (BASIC), INT 19h (Boot).

Каждый отсутствующий вектор — `0xF000:0x0000`, что приведёт к `halt` в необработанном внутреннем прерывании (`interrupts/mod.rs:29-37`).

---

## 13. Новые проблемы (VGA / suspend / display)

### 13.1. [DEAD] `suspend_for_key()` — неиспользуемые поля SuspendedCall
- **Файл:** `system.rs:297-331`
- Поля `cs`, `ip`, `flags`, `sp`, `ss` в `SuspendedCall` всегда устанавливаются в 0 и не используются. Механизм перешёл на re-dispatch через `dispatch(vector)`.

### 13.2. [BUG] `step_frame()` — отступы сломаны
- **Файл:** `system.rs:237-291`
- Тело `step_frame()` имеет неверный отступ (метод на одном уровне с impl-блоком из-за проблем с форматированием после копипасты).

### 13.3. [BUG] `EmuWindow::pump_events()` — паника при закрытом окне
- **Файл:** `gui/src/lib.rs:50-53`
- `pump()` вызывает `window.update_with_buffer()` без проверки `is_open()`. При закрытии окна может упасть.

### 13.4. [STUB] VGA Input Status — нет реального VGA-тайминга
- **Файл:** `peripherals/video.rs:59-78`
- `input_status_1()` эмулирует retrace через wall-clock (Instant::now()), а не через счётчик пикселей/строк. Приемлемо для игр, но не для демок с точным таймингом.

### 13.5. [STUB] VGA CRTC (0x3D4/0x3D5) — не реализован
- **Файл:** `peripherals/video.rs`
- CRTC порты 0x3D4/0x3D5 не обрабатываются (только `log::trace`). Режимы planar полагаются на CRTC-регистры для корректного отображения.

### 13.6. [STUB] Планарные режимы — нет рендеринга с учетом CRTC
- **Файл:** `peripherals/video.rs`
- `render_planar()` выполняет базовую упаковку 4-bit цвета из 4 plane, но без учёта CRTC-регистров (START_ADDR, OFFSET). Режимы 0Eh/10h/12h могут отображаться некорректно.

### 13.7. [BUG] INT 10h/AH=12h — заглушка, возвращает AL=12h
- **Файл:** `interrupts/bios.rs`
- Все подфункции BL=10h-34h возвращают AL=12h ("supported"), но реальной настройки не производят.

### 13.8. [STUB] VGA palette — цвета 16-255 фейковые
- **Файл:** `peripherals/video.rs:default_dac_palette()`
- Indices 32-255 используют приблизительную формулу BIOS ramp, а не реальные VGA-цвета. Для игр и демок, устанавливающих палитру через DAC, это не проблема, но для exact palette reproduction не годится.
