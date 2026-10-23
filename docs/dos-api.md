# DOS API — Поддерживаемые прерывания и функции

## Обозначения

- `[x]` — реализовано полностью
- `[~]` — частично / заглушка
- `[ ]` — не реализовано

## INT 08h — PIT Timer (IRQ0)

- `[x]` Инкремент счётчика времени (BDA 0x046C).
- `[x]` Цепочка на INT 1Ch, если установлен пользовательский вектор.
- `[x]` EOI (0x20 в порт 0x20).

## INT 09h — Keyboard (IRQ1)

- `[~]` Только EOI (0x20 в порт 0x20).
- `[ ]` Не читает scancode из порта 0x60.
- Клавиши поступают через BIOS-буфер напрямую из step_frame().

## INT 10h — Video Services

| AH | Функция | Статус |
|----|---------|--------|
| 00h | Set video mode | `[x]` (03h, 0Dh, 0Eh, 10h, 12h, 13h) |
| 01h | Set cursor shape (CRTC) | `[x]` |
| 02h | Set cursor position | `[x]` (BDA + CRTC sync) |
| 03h | Read cursor position/shape | `[x]` |
| 05h | Select active page | `[~]` (только страница 0) |
| 06h | Scroll up / clear window | `[x]` |
| 07h | Scroll down / clear window | `[x]` |
| 08h | Read character + attribute | `[x]` |
| 09h | Write character + attribute (repeat) | `[x]` |
| 0Ah | Write character only (repeat) | `[x]` |
| 0Eh | Teletype output | `[x]` (Text80x25; в 13h — debug log) |
| 0Fh | Get video mode | `[x]` |
| 12h | Alternate function select | `[~]` (BL=10h+ возвращает AL=12h) |

## INT 12h — Memory Size

`[x]` Возвращает AX = 640 (КБ низкой памяти).

## INT 15h — BIOS Extensions

| AX | Функция | Статус |
|----|---------|--------|
| 2400h | A20 status query | `[x]` |
| 2401h | A20 enable | `[x]` (блокируется через disable_change_flags) |
| 2402h | A20 disable | `[x]` (блокируется через disable_change_flags) |
| 2403h | A20 status (AL=1/0) | `[x]` |
| 8800h..88FFh | Get extended memory | `[~]` (возвращает общий размер — 1MB) |
| прочие | | `[~]` возвращает CF=1, AH=86h |

## INT 16h — Keyboard Services

| AH | Функция | Статус |
|----|---------|--------|
| 00h/10h | Read key (wait, suspend/resume) | `[x]` |
| 01h/11h | Check key (non-blocking) | `[x]` |
| 02h/12h | Get shift flags | `[x]` |
| прочие | | `[~]` лог + unsupported |

## INT 1Ah — Clock

| AH | Функция | Статус |
|----|---------|--------|
| 00h | Read system clock | `[x]` (18.2 Hz ticks от UNIX epoch) |
| 01h | Set system clock | `[~]` (игнорируется) |

## INT 1Ch — User Timer Tick

`[~]` Вызывается из INT 08h, если установлен пользовательский вектор.

## INT 20h — Program Terminate

`[x]` Останавливает CPU (halted = true).

## INT 21h — DOS API

| AH | Функция | Статус | Примечания |
|----|---------|--------|------------|
| 00h | Terminate program | `[x]` | stopped = true |
| 01h | Read char with echo | `[x]` | suspend/resume |
| 02h | Print char (DL) | `[x]` | tty_write() → VRAM + stdout |
| 06h | Direct console I/O | `[x]` | DL=FF: non-blocking read; otherwise write |
| 07h | Direct char input, no echo | `[x]` | blocking, no Ctrl+C |
| 08h | Char input, no echo | `[x]` | blocking, Ctrl+C → INT 23h |
| 09h | Print string until `$` | `[x]` | tty_write() |
| 0Ah | Buffered input | `[x]` | suspend/resume, Backspace, Esc, Ctrl+C |
| 19h | Get current drive | `[x]` | always returns 2 (C:) |
| 1Ah | Set DTA | `[x]` | segment:offset |
| 25h | Set interrupt vector | `[x]` | пишет в IVT |
| 2Ah | Get system date | `[ ]` | |
| 2Ch | Get system time | `[ ]` | |
| 30h | Get DOS version | `[x]` | returns 6.22 |
| 35h | Get interrupt vector | `[x]` | читает из IVT |
| 39h | Create directory (MKDIR) | `[ ]` | |
| 3Ah | Remove directory (RMDIR) | `[ ]` | |
| 3Bh | Change directory (CHDIR) | `[x]` | |
| 3Ch | Create file | `[x]` | |
| 3Dh | Open file | `[x]` | |
| 3Eh | Close file | `[x]` | |
| 3Fh | Read file/handle | `[x]` | CON: suspend/resume, disk: read |
| 40h | Write file/handle | `[x]` | |
| 41h | Delete file | `[x]` | |
| 42h | Seek file | `[x]` | |
| 43h | Get/set file attributes | `[~]` | get: OK, set: stub |
| 44h | IOCTL | `[~]` | только AL=00h (get device info) |
| 47h | Get current directory | `[x]` | возвращает путь для указанного диска |
| 48h | Allocate memory (MCB) | `[x]` | |
| 49h | Free memory (MCB) | `[x]` | |
| 4Ah | Modify memory (MCB) | `[x]` | |
| 4Bh | EXEC (load and execute) | `[ ]` | |
| 4Ch | Terminate with exit code | `[x]` | stopped = true |
| 4Eh | Find First | `[x]` | 8.3 DTA заполнение |
| 4Fh | Find Next | `[x]` | |
| 59h | Get extended error | `[ ]` | |
| 5Ch | File locking | `[ ]` | |
| 71h | Long filename (LFN) | `[ ]` | |
| прочие | | `[~]` | CF=1, AX=1, halted |

### Структура DTA (FindFirst/FindNext)

DTA имеет размер 43 байта (стандарт DOS):

| Смещение | Размер | Поле |
|----------|--------|------|
| 0x15 | 1 | Атрибуты файла |
| 0x16 | 2 | Время (битовое поле DOS) |
| 0x18 | 2 | Дата (битовое поле DOS) |
| 0x1A | 4 | Размер файла |
| 0x1E | 11 | Имя файла (8.3, space-padded, с точкой) |
| 0x29 | 1 | Нулевой терминатор |

### Устройства DOS

Распознаваемые имена устройств (case-insensitive):

| Имя | Назначение |
|-----|-----------|
| CON | Консоль (stdin/stdout) |
| NUL | Null device |
| COM1–COM4 | Последовательные порты |
| LPT1–LPT4 | Параллельные порты |
| PRN | Принтер (LPT1) |
| AUX | Вспомогательное устройство (COM1) |
| CLOCK$ | Системные часы (data reads only) |
| EMMXXXX0 | EMS-устройство |

Хендлы 0–4 предварительно зарегистрированы:

| Handle | Device | Назначение |
|--------|--------|------------|
| 0 | CON | stdin |
| 1 | CON | stdout |
| 2 | CON | stderr |
| 3 | AUX | COM1 |
| 4 | PRN | LPT1 |

## INT 23h — Ctrl+C Handler

`[~]` Вызывается из AH=08h и AH=0Ah при Ctrl+C. Вектор может быть перенаправлен программой через INT 21h/AH=25h. Если обработчик не установлен — программа продолжается.

## INT 2Fh — Multiplex Interrupt

| AX | Функция | Статус |
|----|---------|--------|
| 1687h | DPMI Host Check | `[x]` (зависит от config.dpmi) |
| 0C00h | MS-DOS idle (INT 28h style) | `[x]` (AX=0) |
| AH=16h | Windows/DPMI functions | `[~]` CF=1 |
| AH=4Ah | TSR query | `[~]` AL=0 |
| AH=12h | Network query | `[~]` AL=0 |
| AH=0Ch | Mouse query | `[~]` AL=0 |
| AH=0Dh | Printer query | `[~]` AL=0 |

## INT 67h — EMS (LIM EMS 4.0)

| AH | Функция | Статус | Примечания |
|----|---------|--------|------------|
| 40h | Get Status | `[x]` | |
| 41h | Get Page Frame Segment | `[x]` | возвращает 0xD000 |
| 42h | Map/Unmap Page | `[~]` | проверяет handle, не маппит реально |
| 43h | Get Unallocated Pages | `[x]` | 256 свободных |
| 44h | Allocate Pages | `[x]` | |
| 45h | Deallocate Pages | `[x]` | |
| 46h | Get Version | `[x]` | 4.0 |
| 4Bh | Get Handle Count | `[x]` | |
| 4Ch | Get Handle Pages | `[x]` | |
| 4Dh | Get All Handle Pages | `[x]` | |
| 4Eh | Get/Set Page Map | `[~]` | размер/сохранение карты |
| прочие | | `[~]` | AH=0x84, CF=1 |

Включается через `machine.ems = true` в config.toml.

## IVT — Векторы прерываний

Все внутренние обработчики имеют CS=0xF000 и живут в Rust-коде:

| Вектор | CS | IP | Обработчик |
|--------|----|----|-----------|
| 08h | F000 | 0010 | INT 08h — PIT Timer |
| 09h | F000 | 0020 | INT 09h — Keyboard (EOI) |
| 10h | F000 | 0030 | INT 10h — Video Services |
| 12h | F000 | 0040 | INT 12h — Memory Size |
| 13h | F000 | 0060 | INT 13h — Disk (stub) |
| 15h | F000 | 0070 | INT 15h — BIOS Extensions |
| 16h | F000 | 0080 | INT 16h — Keyboard |
| 1Ah | F000 | 0050 | INT 1Ah — Clock |
| 1Ch | F000 | 0090 | INT 1Ch — User Timer |
| 1Fh | F000 | 00A0 | INT 1Fh — Video Graphics Table |
| 20h | F000 | 00B0 | INT 20h — Program Terminate |
| 21h | F000 | 00C0 | INT 21h — DOS API |
| 2Fh | F000 | 00D0 | INT 2Fh — Multiplex |
| 67h | F000 | 0100 | INT 67h — EMS 4.0 |

Вектора с CS≠0xF000 — установленные программой — вызываются через far call на адрес из IVT.

## Suspend/Resume — Неблокирующий ввод

Функции, ожидающие клавишу (INT 21h/AH=01h, 07h, 08h, 0Ah; INT 21h/AH=3Fh handle=0; INT 16h/AH=00h/10h):

1. Если буфер клавиатуры пуст — вызов `sys.suspend_for_key(vector)` сохраняет вектор прерывания.
2. В `step_frame()` обнаруживается `suspended`, вызывается `pump_events()`, окно обновляется во время ожидания.
3. При появлении клавиши — `dispatch(vector)` перезапускает обработку прерывания.
4. Теперь клавиша есть в буфере — функция завершается нормально.

Механизм re-dispatch (cs/ip/sp/ss/flags не используются — упрощение без stack-unwind).
