# Перенос опкодов в `crate/x86`

Рабочий эталон семантики — `libs/dos_core`. Новый CPU — `crate/x86` (`X86Cpu` + `bus::Machine`). Старый обработчик не удалять, пока `rusn_dos run` ещё идёт старым циклом. Бинарник не переключается по одному байту.

При реализации этот файл копируется в `TODO.md` и заменяет его целиком. Старые `TODO.md` и `todo.todo` не использовать.

## Граница

Регистры, флаги, IP, префиксы, GDTR, IDTR и CR0/CR3 живут только в `X86Cpu`. Память, A20, порты, видео, ФС, EMS и MCB живут в машине. В одном запуске программы второй файл регистров не используется и не копируется.

`DosMachine.registers` остаётся только у старого `cpu/run.rs`. `X86Cpu` и `DosMachine` не исполняют соседние опкоды одного прогона.

## Правила одного опкода

- Сигнатура: `fn name(cpu: &mut X86Cpu, machine: &mut dyn Machine)`. Без `DosMachine` и без `prev`.
- Байт опкода один. Префикс `66h` — та же ветка, операнд 32 бита. Префикс `67h` — та же ветка, адрес 32 бита. Отдельным пунктом `66h` не заводить.
- Семантику брать из `libs/dos_core/src/instructions/` и поддиспетчеров `cpu/execute/`. Если там ветки нет или она ведёт в `halted`, реализовывать по 8086/386, а не копировать дыру.
- Тест: байты в `Motherboard`, `X86Cpu::step`, проверка регистров CPU, флагов, памяти и IP. Не через `DosMachine::run`. Для арифметики проверять CF, OF, SF, ZF, AF, PF. Для `INC`/`DEC` CF не меняется.
- В `crate/x86/src/executor.rs` одна ветка на байт. Неизвестный байт останавливает CPU и пишет опкод и CS:IP. Не прыгать в `INT 06h`, пока это не отдельный пункт.
- `INT` на `Motherboard` идёт по IVT: хук возвращает `false`. DOS/BIOS к тесту опкода не подключать.

## 0. До первого полного запуска программы

Опкоды разделов 1–9 можно переносить и тестировать на `Motherboard` без этого раздела. Раздел обязателен только перед переключением `rusn_dos run` конкретной программы на `X86Cpu`.

- [ ] Удалить `libs/dos_core/src/hybrid_run.rs`. Побайтового моста не делать.
- [ ] В `bus::Machine` заменить `handle_software_interrupt(&mut self, vector: u8) -> bool` на передачу `regs: &mut CpuRegs`. Поля: AX..DI, SP, CS, DS, ES, SS, FS, GS, IP, FLAGS и EAX..EDI. Хук не принимает `X86Cpu`.
- [ ] `true` — обработчик прочитал AH/флаги из `regs` и записал результат туда же. CPU кадр FLAGS/CS/IP не делает.
- [ ] `false` — CPU делает кадр и прыжок по IVT, как уже написано в `crate/x86/src/instructions/system.rs`.
- [ ] `Motherboard` возвращает `false`.
- [ ] `DosMachine` возвращает `true` только если в IVT у вектора CS=`F000` и вектор известен (`08h`, `10h`, `12h`, `15h`, `16h`, `1Ah`, `1Ch`, `20h`, `21h`, `2Fh`, `67h`). Иначе `false`.
- [ ] `handle_int21` и остальные внутренние обработчики читают и пишут `CpuRegs`. Не копировать `regs` в `DosMachine.registers` на время вызова.
- [ ] Память, видео, ФС и EMS обработчик по-прежнему берёт из `DosMachine`. Меняется только доступ к регистрам CPU.
- [ ] `in_port` / `out_port` у `DosMachine` вызывают существующую логику портов из `instructions/system.rs`, а не пустую заглушку. A20 остаётся полем машины.
- [ ] Стартовые CS:IP, SS:SP, DS, ES после загрузки COM/EXE писать в `X86Cpu`, не в `DosMachine.registers`.
- [ ] `rusn_dos run` остаётся на старом цикле. Отдельный флаг запускает весь прогон через `X86Cpu::step`. Флаг включать для программы только когда каждый байт из её лога инструкций отмечен в этом файле.

## 1. Префиксы

Не инструкции. Уже читаются в `X86Cpu::step`; проверить, что они не попадают в диспетчер опкода.

- [ ] `26h` ES, `2Eh` CS, `36h` SS, `3Eh` DS, `64h` FS, `65h` GS
- [ ] `66h` operand size, `67h` address size
- [ ] `F0h` LOCK — запомнить и игнорировать, как сейчас в `dos_core`
- [ ] `F2h` REPNZ, `F3h` REPZ — действуют на строковые и портовые пункты ниже
- [ ] `0Fh` — переход в отдельный диспетчер раздела 9

## 2. ALU

Каждый пункт: r/m и регистр, память, immediate; плюс форма `66h`, если она указана.

- [ ] `00h` ADD r/m8, r8
- [ ] `01h` ADD r/m16, r16; `66h` — r/m32, r32
- [ ] `02h` ADD r8, r/m8
- [ ] `03h` ADD r16, r/m16; `66h` — r32, r/m32
- [ ] `04h` ADD AL, imm8
- [ ] `05h` ADD AX, imm16; `66h` — EAX, imm32
- [ ] `08h` OR r/m8, r8
- [ ] `09h` OR r/m16, r16; `66h` — r/m32, r32
- [ ] `0Ah` OR r8, r/m8
- [ ] `0Bh` OR r16, r/m16; `66h` — r32, r/m32
- [ ] `0Ch` OR AL, imm8
- [ ] `0Dh` OR AX, imm16; `66h` — EAX, imm32
- [ ] `10h` ADC r/m8, r8
- [ ] `11h` ADC r/m16, r16; `66h` — r/m32, r32
- [ ] `12h` ADC r8, r/m8
- [ ] `13h` ADC r16, r/m16; `66h` — r32, r/m32
- [ ] `14h` ADC AL, imm8
- [ ] `15h` ADC AX, imm16; `66h` — EAX, imm32. В старом диспетчере байта нет (`10h..=14h`)
- [ ] `18h` SBB r/m8, r8
- [ ] `19h` SBB r/m16, r16; `66h` — r/m32, r32
- [ ] `1Ah` SBB r8, r/m8. В старом диспетчере байта нет
- [ ] `1Bh` SBB r16, r/m16; `66h` — r32, r/m32
- [ ] `1Ch` SBB AL, imm8. В старом диспетчере байта нет
- [ ] `1Dh` SBB AX, imm16; `66h` — EAX, imm32. В старом диспетчере байта нет
- [ ] `20h` AND r/m8, r8
- [ ] `21h` AND r/m16, r16; `66h` — r/m32, r32
- [ ] `22h` AND r8, r/m8
- [ ] `23h` AND r16, r/m16; `66h` — r32, r/m32
- [ ] `24h` AND AL, imm8
- [ ] `25h` AND AX, imm16; `66h` — EAX, imm32. В старом `and()` ветки нет
- [ ] `27h` DAA
- [ ] `28h` SUB r/m8, r8. В старом `sub()` ветки нет
- [ ] `29h` SUB r/m16, r16; `66h` — r/m32, r32
- [ ] `2Ah` SUB r8, r/m8
- [ ] `2Bh` SUB r16, r/m16; `66h` — r32, r/m32
- [ ] `2Ch` SUB AL, imm8
- [ ] `2Dh` SUB AX, imm16; `66h` — EAX, imm32. В старом диспетчере байта нет
- [ ] `2Fh` DAS
- [ ] `30h` XOR r/m8, r8
- [ ] `31h` XOR r/m16, r16; `66h` — r/m32, r32
- [ ] `32h` XOR r8, r/m8
- [ ] `33h` XOR r16, r/m16; `66h` — r32, r/m32
- [ ] `34h` XOR AL, imm8. В старом `xor()` ветки нет
- [ ] `35h` XOR AX, imm16; `66h` — EAX, imm32. В старом `xor()` ветки нет
- [ ] `38h` CMP r/m8, r8
- [ ] `39h` CMP r/m16, r16; `66h` — r/m32, r32
- [ ] `3Ah` CMP r8, r/m8
- [ ] `3Bh` CMP r16, r/m16; `66h` — r32, r/m32
- [ ] `3Ch` CMP AL, imm8
- [ ] `3Dh` CMP AX, imm16; `66h` — EAX, imm32
- [ ] `40h` INC AX; `66h` — EAX. CF не менять
- [ ] `41h` INC CX; `66h` — ECX
- [ ] `42h` INC DX; `66h` — EDX
- [ ] `43h` INC BX; `66h` — EBX
- [ ] `44h` INC SP; `66h` — ESP
- [ ] `45h` INC BP; `66h` — EBP
- [ ] `46h` INC SI; `66h` — ESI
- [ ] `47h` INC DI; `66h` — EDI
- [ ] `48h` DEC AX; `66h` — EAX
- [ ] `49h` DEC CX; `66h` — ECX
- [ ] `4Ah` DEC DX; `66h` — EDX
- [ ] `4Bh` DEC BX; `66h` — EBX
- [ ] `4Ch` DEC SP; `66h` — ESP
- [ ] `4Dh` DEC BP; `66h` — EBP
- [ ] `4Eh` DEC SI; `66h` — ESI
- [ ] `4Fh` DEC DI; `66h` — EDI

## 3. Стек и обмен

Стек только через SS:SP, сегментный префикс не меняет SS.

- [ ] `06h` PUSH ES
- [ ] `07h` POP ES
- [ ] `0Eh` PUSH CS
- [ ] `16h` PUSH SS. В старом диспетчере байта нет
- [ ] `17h` POP SS и запрет прерывания на одну инструкцию. В старом диспетчере байта нет
- [ ] `1Eh` PUSH DS
- [ ] `1Fh` POP DS
- [ ] `50h` PUSH AX; `66h` — EAX
- [ ] `51h` PUSH CX; `66h` — ECX
- [ ] `52h` PUSH DX; `66h` — EDX
- [ ] `53h` PUSH BX; `66h` — EBX
- [ ] `54h` PUSH SP; `66h` — ESP. Значение SP — до уменьшения указателя
- [ ] `55h` PUSH BP; `66h` — EBP
- [ ] `56h` PUSH SI; `66h` — ESI
- [ ] `57h` PUSH DI; `66h` — EDI
- [ ] `58h` POP AX; `66h` — EAX
- [ ] `59h` POP CX; `66h` — ECX
- [ ] `5Ah` POP DX; `66h` — EDX
- [ ] `5Bh` POP BX; `66h` — EBX
- [ ] `5Ch` POP SP; `66h` — ESP
- [ ] `5Dh` POP BP; `66h` — EBP
- [ ] `5Eh` POP SI; `66h` — ESI
- [ ] `5Fh` POP DI; `66h` — EDI
- [ ] `60h` PUSHA; `66h` — PUSHAD
- [ ] `61h` POPA; `66h` — POPAD
- [ ] `68h` PUSH imm16; `66h` — imm32
- [ ] `6Ah` PUSH imm8 sign-extend. В старом диспетчере байта нет
- [ ] `8Fh` POP r/m16; `66h` — POP r/m32. Сейчас реализован только 16-битный путь
- [ ] `9Ch` PUSHF; `66h` — PUSHFD
- [ ] `9Dh` POPF; `66h` — POPFD
- [ ] `86h` XCHG r/m8, r8
- [ ] `87h` XCHG r/m16, r16; `66h` — r/m32, r32
- [ ] `91h` XCHG CX, AX; `66h` — ECX, EAX
- [ ] `92h` XCHG DX, AX; `66h` — EDX, EAX
- [ ] `93h` XCHG BX, AX; `66h` — EBX, EAX
- [ ] `94h` XCHG SP, AX; `66h` — ESP, EAX
- [ ] `95h` XCHG BP, AX; `66h` — EBP, EAX
- [ ] `96h` XCHG SI, AX; `66h` — ESI, EAX
- [ ] `97h` XCHG DI, AX; `66h` — EDI, EAX

## 4. Пересылка

- [ ] `88h` MOV r/m8, r8. Ветка в `crate/x86` есть — сверить и закрыть тестом
- [ ] `89h` MOV r/m16, r16; `66h` — r/m32, r32. 32-бит сейчас делает halt
- [ ] `8Ah` MOV r8, r/m8. Ветка есть — сверить и закрыть тестом
- [ ] `8Bh` MOV r16, r/m16; `66h` — r32, r/m32. 32-бит сейчас делает halt
- [ ] `8Ch` MOV r/m16, sreg
- [ ] `8Dh` LEA r16, m; `66h` — LEA r32, m
- [ ] `8Eh` MOV sreg, r/m16. Запись в SS запрещает прерывание на одну инструкцию
- [ ] `A0h` MOV AL, moffs. Учесть `67h`
- [ ] `A1h` MOV AX, moffs; комбинации `66h`/`67h` дают AX/EAX и moffs16/moffs32
- [ ] `A2h` MOV moffs, AL
- [ ] `A3h` MOV moffs, AX; `66h` — moffs, EAX
- [ ] `B0h` MOV AL, imm8. Ветка есть — закрыть тестом
- [ ] `B1h` MOV CL, imm8
- [ ] `B2h` MOV DL, imm8
- [ ] `B3h` MOV BL, imm8
- [ ] `B4h` MOV AH, imm8
- [ ] `B5h` MOV CH, imm8
- [ ] `B6h` MOV DH, imm8
- [ ] `B7h` MOV BH, imm8
- [ ] `B8h` MOV AX, imm16; `66h` — EAX, imm32
- [ ] `B9h` MOV CX, imm16; `66h` — ECX, imm32. Сейчас 32-бит не разобран, в отличие от `B8h`
- [ ] `BAh` MOV DX, imm16; `66h` — EDX, imm32
- [ ] `BBh` MOV BX, imm16; `66h` — EBX, imm32
- [ ] `BCh` MOV SP, imm16; `66h` — ESP, imm32
- [ ] `BDh` MOV BP, imm16; `66h` — EBP, imm32
- [ ] `BEh` MOV SI, imm16; `66h` — ESI, imm32
- [ ] `BFh` MOV DI, imm16; `66h` — EDI, imm32
- [ ] `C4h` LES r16, m16:32
- [ ] `C5h` LDS r16, m16:32
- [ ] `C6h` MOV r/m8, imm8
- [ ] `C7h` MOV r/m16, imm16; `66h` — r/m32, imm32

## 5. Строковые и портовые строки

DF выбирает шаг +N или −N. `F3h` повторяет, пока CX/ECX не ноль. Оптимизация видеопамяти остаётся в машине, не в CPU.

- [ ] `A4h` MOVSB; `F3h` — REP MOVSB
- [ ] `A5h` MOVSW; `66h` — MOVSD; `F3h` — REP
- [ ] `A6h` CMPSB; `F3h` — REPE, `F2h` — REPNE
- [ ] `A7h` CMPSW; `66h` — CMPSD; `F3h`/`F2h`
- [ ] `AAh` STOSB; `F3h` — REP STOSB
- [ ] `ABh` STOSW; `66h` — STOSD; `F3h`
- [ ] `ACh` LODSB; `F3h`
- [ ] `ADh` LODSW; `66h` — LODSD. В старом диспетчере байта нет, есть только `ACh` и `AEh`
- [ ] `AEh` SCASB; `F3h`/`F2h`
- [ ] `AFh` SCASW; `66h` — SCASD; `F3h`/`F2h`
- [ ] `6Ch` INSB; `F3h` — REP INSB
- [ ] `6Dh` INSW; `66h` — INSD; `F3h`
- [ ] `6Eh` OUTSB; `F3h`
- [ ] `6Fh` OUTSW; `66h` — OUTSD; `F3h`

## 6. Управление

`70h..=7Fh` — rel8. Условие смотрит флаги, переход прибавляет rel8 к IP уже после байта смещения.

- [ ] `70h` JO
- [ ] `71h` JNO
- [ ] `72h` JB / JC
- [ ] `73h` JAE / JNC
- [ ] `74h` JE / JZ
- [ ] `75h` JNE / JNZ
- [ ] `76h` JBE
- [ ] `77h` JA
- [ ] `78h` JS
- [ ] `79h` JNS
- [ ] `7Ah` JP / JPE. Сейчас `70h..=7Fh` ведёт в `jumps()`, а ветки нет — CPU останавливается
- [ ] `7Bh` JNP / JPO. Та же дыра
- [ ] `7Ch` JL
- [ ] `7Dh` JGE
- [ ] `7Eh` JLE
- [ ] `7Fh` JG
- [ ] `E0h` LOOPNZ rel8. Уменьшает CX, переход если CX≠0 и ZF=0
- [ ] `E1h` LOOPZ rel8. Переход если CX≠0 и ZF=1
- [ ] `E2h` LOOP rel8. Переход если CX≠0
- [ ] `E3h` JCXZ rel8; `66h` — JECXZ
- [ ] `E8h` CALL rel16; `66h` — CALL rel32
- [ ] `E9h` JMP rel16; `66h` — JMP rel32. Сейчас `66h` не разобран
- [ ] `EAh` JMP ptr16:16
- [ ] `EBh` JMP rel8
- [ ] `9Ah` CALL ptr16:16
- [ ] `C2h` RET imm16; `66h` — RET, кадр 32 бита
- [ ] `C3h` RETN; `66h` — кадр 32 бита
- [ ] `CAh` RETF imm16. В старом диспетчере байта нет
- [ ] `CBh` RETF
- [ ] `C8h` ENTER. В старом диспетчере байта нет
- [ ] `C9h` LEAVE
- [ ] `62h` BOUND r16, m; `66h` — BOUND r32, m

## 7. Системные и прочие однобайтные

- [ ] `90h` NOP. Уже есть — закрыть тестом. `91h..=97h` это XCHG, не NOP
- [ ] `98h` CBW; `66h` — CWDE
- [ ] `99h` CWD; `66h` — CDQ
- [ ] `9Bh` WAIT. Сейчас пустая инструкция, так и переносить
- [ ] `9Eh` SAHF
- [ ] `9Fh` LAHF
- [ ] `CCh` INT3
- [ ] `CDh` INT imm8. Сначала `handle_software_interrupt`, иначе кадр FLAGS/CS/IP и сброс IF, TF
- [ ] `CEh` INTO. В старом диспетчере байта нет
- [ ] `CFh` IRET. Уже есть — сверить порядок IP, CS, FLAGS
- [ ] `E4h` IN AL, imm8
- [ ] `E5h` IN AX, imm8; `66h` — EAX. В старом диспетчере байта нет
- [ ] `E6h` OUT imm8, AL
- [ ] `E7h` OUT imm8, AX; `66h` — EAX. В старом диспетчере байта нет
- [ ] `ECh` IN AL, DX
- [ ] `EDh` IN AX, DX; `66h` — EAX. В старом диспетчере байта нет
- [ ] `EEh` OUT DX, AL. В старом диспетчере байта нет
- [ ] `EFh` OUT DX, AX; `66h` — EAX. В старом диспетчере байта нет
- [ ] `F4h` HLT
- [ ] `F5h` CMC
- [ ] `F8h` CLC
- [ ] `F9h` STC
- [ ] `FAh` CLI
- [ ] `FBh` STI
- [ ] `FCh` CLD
- [ ] `FDh` STD
- [ ] `63h` ARPL
- [ ] `69h` IMUL r16, r/m16, imm16; `66h` — r32, r/m32, imm32
- [ ] `6Bh` IMUL r16, r/m16, imm8. В старом диспетчере байта нет
- [ ] `D4h` AAM. В старом диспетчере байта нет
- [ ] `D5h` AAD. В старом диспетчере байта нет
- [ ] `D6h` SALC. В старом диспетчере байта нет
- [ ] `D7h` XLAT. В старом диспетчере байта нет
- [ ] `F1h` INT1. Не реализовывать, пока нет потребителя; байт должен явно останавливать CPU, а не попадать в `_`

## 8. Группы ModR/M

Один пункт — один `/reg`. Память и регистр проверять отдельно.

### `80h` r/m8, imm8

- [ ] `80h /0` ADD
- [ ] `80h /1` OR
- [ ] `80h /2` ADC
- [ ] `80h /3` SBB
- [ ] `80h /4` AND
- [ ] `80h /5` SUB
- [ ] `80h /6` XOR
- [ ] `80h /7` CMP

### `81h` r/m16, imm16; `66h` — r/m32, imm32

- [ ] `81h /0` ADD
- [ ] `81h /1` OR
- [ ] `81h /2` ADC
- [ ] `81h /3` SBB
- [ ] `81h /4` AND
- [ ] `81h /5` SUB
- [ ] `81h /6` XOR
- [ ] `81h /7` CMP

### `82h`

- [ ] `82h /0../7` — алиас `80h` для 8086. В старом диспетчере байта нет

### `83h` r/m16, imm8 sign-extend; `66h` — r/m32

- [ ] `83h /0` ADD
- [ ] `83h /1` OR
- [ ] `83h /2` ADC
- [ ] `83h /3` SBB
- [ ] `83h /4` AND
- [ ] `83h /5` SUB
- [ ] `83h /6` XOR
- [ ] `83h /7` CMP

### Сдвиги

`C0h` r/m8, imm8. `C1h` r/m16, imm8; `66h` — r/m32. `D0h` r/m8, 1. `D1h` r/m16, 1; `66h` — r/m32. `D2h` r/m8, CL. `D3h` r/m16, CL; `66h` — r/m32. Count маскировать: 8-бит — 5 младших бит для `C0h`/`D2h` по правилам 80186+, для `D0h`/`D1h` счётчик всегда 1. CF и OF для count=1 сверять отдельно от count>1.

- [ ] `/0` ROL на `C0h`, `C1h`, `D0h`, `D1h`, `D2h`, `D3h`
- [ ] `/1` ROR на тех же шести байтах
- [ ] `/2` RCL на тех же шести байтах
- [ ] `/3` RCR на тех же шести байтах
- [ ] `/4` SHL на тех же шести байтах
- [ ] `/5` SHR на тех же шести байтах
- [ ] `/6` SHL, тот же код что `/4`, не отдельная операция
- [ ] `/7` SAR на тех же шести байтах

### `F6h` r/m8

- [ ] `F6h /0` TEST imm8
- [ ] `F6h /1` TEST imm8, алиас `/0`
- [ ] `F6h /2` NOT
- [ ] `F6h /3` NEG
- [ ] `F6h /4` MUL
- [ ] `F6h /5` IMUL
- [ ] `F6h /6` DIV
- [ ] `F6h /7` IDIV. Деление на ноль — `INT 00h`, не panic

### `F7h` r/m16; `66h` — r/m32

- [ ] `F7h /0` TEST imm16; `66h` — imm32
- [ ] `F7h /1` TEST, алиас `/0`
- [ ] `F7h /2` NOT
- [ ] `F7h /3` NEG
- [ ] `F7h /4` MUL
- [ ] `F7h /5` IMUL
- [ ] `F7h /6` DIV
- [ ] `F7h /7` IDIV

### `FEh`

- [ ] `FEh /0` INC r/m8
- [ ] `FEh /1` DEC r/m8
- [ ] `FEh /2../7` — не определено, останавливать CPU

### `FFh`

- [ ] `FFh /0` INC r/m16; `66h` — r/m32
- [ ] `FFh /1` DEC r/m16; `66h` — r/m32
- [ ] `FFh /2` CALL r/m16; `66h` — r/m32
- [ ] `FFh /3` CALL m16:16
- [ ] `FFh /4` JMP r/m16; `66h` — r/m32
- [ ] `FFh /5` JMP m16:16
- [ ] `FFh /6` PUSH r/m16; `66h` — r/m32
- [ ] `FFh /7` не определено, останавливать CPU

## 9. `0Fh`

Только то, что есть в `execute_0f`, плюс явный отказ для остального. Новые `0F xx` не добавлять, пока их нет в тестовых COM/EXE.

- [ ] `0Fh 01h /0` SGDT
- [ ] `0Fh 01h /1` SIDT
- [ ] `0Fh 01h /2` LGDT. Пишет `gdtr_limit` и `gdtr_base` в `X86Cpu`, не в `DosMachine`
- [ ] `0Fh 01h /3` LIDT. Пишет `idtr_limit` и `idtr_base` в `X86Cpu`
- [ ] `0Fh 01h /4` SMSW
- [ ] `0Fh 01h /5../7` — как сейчас: предупреждение и пропуск адреса, без halt
- [ ] `0Fh 20h` MOV r32, CRn
- [ ] `0Fh 22h` MOV CRn, r32
- [ ] `0Fh 82h` JB rel16; `66h` — rel32
- [ ] `0Fh 83h` JAE rel16; `66h` — rel32
- [ ] `0Fh 84h` JE rel16; `66h` — rel32
- [ ] `0Fh A1h` POP FS
- [ ] `0Fh B7h` MOVZX r16, r/m8; `66h` — MOVZX r32, r/m16
- [ ] Любой другой `0Fh xx` останавливает CPU с логом. Не вызывать `INT 06h` молча, как сейчас делает старый `_`

## 10. Когда пункт можно вычеркнуть

- [ ] Тест опкода зелёный в `cargo test -p x86` на `Motherboard` и `X86Cpu`.
- [ ] Для опкода, который уже есть в старом диспетчере: второй изолированный прогон старого CPU с той же памятью и теми же начальными регистрами даёт те же регистры, флаги, память и IP. Общего цикла нет. Для дыр старого диспетчера эталон — тест по 8086/386.
- [ ] `cargo check --workspace` без новых ошибок.
- [ ] `./target/debug/rusn_dos run ./test/test1` без нового флага по-прежнему идёт старым CPU.
