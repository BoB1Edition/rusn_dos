# План исправления "Not enough low memory. 500 KB needed"

## Проблема

BIPRO.EXE/B.EXE выдаёт "Memory error: Not enough low memory. 500 KB needed" — INT 21h/AH=48h не может выделить память.

## Корневые причины (три независимых бага)

### Причина 1 (MCB chain — основная): PSP не включён в размер блока MCB

**Файл:** `crate/dos/src/loader/exe_loader.rs`, строка 124

```
PSP_SEGMENT = 0xFF0  (LOAD_SEGMENT - 0x10, 16 параграфов)
LOAD_SEGMENT = 0x1000
```

**Текущий код:**
```rust
prog_mcb = MCB { size: prog_paragraphs };                        // ❌ только код
prog_mcb.write(PSP_SEGMENT - 1);                                  // MCB на 0xFEF
// data block: 0xFF0 .. 0xFF0 + prog_paragraphs - 1
// next seg:   0xFEF + 1 + prog_paragraphs = 0xFF0 + prog_paragraphs
free_mcb.write(LOAD_SEGMENT + prog_paragraphs);                   // free_mcb на 0x1000 + prog_paragraphs
```

**Разрыв:** allocator ищет следующий MCB по адресу `0xFF0 + prog_paragraphs`, а free_mcb лежит в `0x1000 + prog_paragraphs`. Разница — 0x10 параграфов (256 байт PSP не учтены). allocator читает мусор/нули, не находит `MCB_SIGNATURE_LAST`, не находит свободного блока → возвращает None → ошибка "Not enough memory".

**Исправление:**
```rust
prog_mcb = MCB { size: 0x10 + prog_paragraphs };                  // ✅ PSP + код
// data block: 0xFF0 .. 0xFF0 + 0x10 + prog_paragraphs - 1 = 0x1000 + prog_paragraphs - 1
// next seg:   0xFEF + 1 + 0x10 + prog_paragraphs = 0x1000 + prog_paragraphs = end_segment ✓
```

### Причина 2 (SIB modrm): EBP suppressed использует DS вместо SS

**Файл:** `crate/x86/src/modrm.rs`, строка 121

SIB `base_field == 5 && mod_field == 0` (`[scaled_index + disp32]`, EBP подавлен).

В реальном x86 сегмент по умолчанию — **SS** (EBP всегда привязан к SS).  
В старом коде (`b2a2d37`) было `machine.registers.ss()`.  
При рефакторинге стало `cpu.registers.ds()`.

```rust
// Было (регрессия):
let seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());

// Стало:
let seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ss());
```

### Причина 3 (non-SIB modrm — отсутствует проверка rm_field==4)

**Файл:** `crate/x86/src/modrm.rs`, строка 156

Хотя `rm_field == 4` де-факто не может попасть в не-SIB путь, старый код содержал эту проверку для консистентности. Сейчас:

```rust
// Было:
ss_default = ss_default || self.rm_field == 5;

// Стало (как было в старом коде):
ss_default = ss_default || self.rm_field == 4 || self.rm_field == 5;
```

## Все изменения

| Файл | Строка | Было | Стало |
|---|---|---|---|
| `crate/dos/src/loader/exe_loader.rs` | 124 | `size: prog_paragraphs` | `size: 0x10 + prog_paragraphs` |
| `crate/x86/src/modrm.rs` | 121 | `unwrap_or(cpu.registers.ds())` | `unwrap_or(cpu.registers.ss())` |
| `crate/x86/src/modrm.rs` | 156 | `\|\| self.rm_field == 5` | `\|\| self.rm_field == 4 \|\| self.rm_field == 5` |

## Порядок

1. `crate/dos/src/loader/exe_loader.rs:124` — `prog_paragraphs` → `0x10 + prog_paragraphs`
2. `crate/x86/src/modrm.rs:121` — `ds()` → `ss()`
3. `crate/x86/src/modrm.rs:156` — добавить `rm_field == 4`
4. `cargo build`
5. `cargo test`
6. `cargo run -- run app/biing/B.EXE`
7. Проверить лог — allocate должен вернуть Some
