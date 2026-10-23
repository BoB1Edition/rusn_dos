; ============================================================
;  TESTEMU.ASM — комплексный тест эмулятора DOS (COM-файл)
; ============================================================
;  Сборка:
;     TASM TESTEMU.ASM
;     TLINK /t TESTEMU.OBJ        ; /t — собрать как COM
;  На выходе: TESTEMU.COM
;
;  Ожидания:
;     [1]  06.16         [2]  640 KB        [3]  C:
;     [4]  пустой каталог (ASCIZ!)          [5]  Hello from teletype!
;     [6]  AC (BS перезаписал B)            [7]  OK + "HI" в (0,75)
;     [8]  меняющееся значение 3DAh         [9]  03
;     [10] Disabled     [11] Not installed  [12] имя файла (ASCIZ!)
;     [13] CON device write OK!             [14] Device
;     [15] no key       [16] Zno key
;     [17] suspend, ждёт клавишу            [18] suspend, ждёт строку
;     [19] синий экран + КВАДРАТНЫЙ красный квадрат, ждёт клавишу
;     [20] suspend, читает клавишу через хендл CON
; ============================================================

.MODEL TINY
.CODE
ORG 100h

START:
    JMP MAIN

; ============================================================
; ДАННЫЕ
; ============================================================
MSG_TITLE   DB '=== EMULATOR TEST SUITE v1.1 ===', 0Dh, 0Ah, '$'
MSG_PAUSE   DB 'Press any key to exit...', 0Dh, 0Ah, '$'
MSG_CRLF    DB 0Dh, 0Ah, '$'
MSG_OK      DB 'OK', 0Dh, 0Ah, '$'
MSG_AC      DB '[26] AttrCtrl reg1->DAC4 (ожид. 04, красный A): $'
MSG_FAIL    DB 'FAIL', 0Dh, 0Ah, '$'
MSG_MODE12  DB '[24] Mode 12h planar - press key...$', 0Dh, 0Ah, '$'
MSG_DOSVER  DB '[1] DOS Version: $'
MSG_MEMORY  DB '[2] Memory (INT 12h): $'
MSG_KB      DB ' KB$'
MSG_DRIVE   DB '[3] Current drive: $'
MSG_DIR     DB '[4] Current dir: $'
MSG_TTY     DB '[5] Teletype: $'
MSG_BS      DB '[6] Backspace: AB', 08h, 'C$'
MSG_DIRECT  DB '[7] Direct B800h write: $'
MSG_PORT    DB '[8] Port 3DAh: $'
MSG_VIDEO   DB '[9] Video mode: $'
MSG_A20     DB '[10] A20: $'
MSG_A20_ON  DB 'Enabled$'
MSG_A20_OFF DB 'Disabled$'
MSG_DPMI    DB '[11] DPMI: $'
MSG_DPMI_Y  DB 'Installed$'
MSG_DPMI_N  DB 'Not installed$'
MSG_FIND    DB '[12] FindFirst: $'
MSG_CON     DB '[13] CON write: $'
MSG_IOCTL   DB '[14] IOCTL stdin: $'
MSG_DEV_Y   DB 'Device$'
MSG_DEV_N   DB 'Not a device$'
MSG_KEY1    DB '[15] INT 16h/AH=01h: $'
MSG_KEY_YES DB 'key pending$'
MSG_KEY_NO  DB 'no key$'
MSG_KEY6    DB '[16] INT 21h/AH=06h: $'
MSG_KEYWAIT DB '[17] Press any key (INT 16h/AH=00h)...', 0Dh, 0Ah, '$'
MSG_GOTKEY  DB '     Got: $'
MSG_BUFIN   DB '[18] Type a line (AH=0Ah): $'
MSG_MODE13  DB '[19] Mode 13h - press key to return...', 0Dh, 0Ah, '$'
MSG_CONREAD DB '[20] Read from CON (AH=3Fh) - press key: $'
MSG_DONE    DB 0Dh, 0Ah, '=== ALL TESTS COMPLETE ===', 0Dh, 0Ah, '$'
MSG_CURSOR  DB '[21] Cursor set/get (ожид. 0A14): $'
MSG_CELL    DB '[22] Cell write/read (ожид. 5A): $'
MSG_SCROLL  DB '[23] Scroll/clear region: $'
MSG_PAGE    DB '[25] CRTC page flip - press key...$', 0Dh, 0Ah, '$'
PAGE2_STR   DB 'PAGE2', 0
MSG_VBE       DB 0Dh,0Ah,'[27] VBE 640x480x256: press any key...$',0Dh,0Ah
MSG_VBE_FAIL  DB 'VBE set mode failed!',0Dh,0Ah,'$'
VBE_INFO      EQU $ - 256
VBE_BUF       DB 256 DUP(0)       ; зарезервируй 256 байт где-нибудь в .data (например DB 256 DUP(0))

; ASCIZ-строки: печатаются ТОЛЬКО через PRINT_ASCIZ, не через AH=09h!
TTY_STR     DB 'Hello from teletype!', 0
FIND_MASK   DB '*.*', 0
CON_NAME    DB 'CON', 0
CON_MSG     DB 'CON device write OK!'
CON_MSG_LEN EQU $ - CON_MSG

; Буферы
INPUT_BUF   DB 80               ; макс. символов
            DB ?                ; реальное кол-во (пишет DOS)
            DB 80 DUP(?)        ; символы
DIR_BUF     DB 64 DUP(0)
DTA_BUF     DB 43 DUP(0)
READ_BUF    DB 0

; ============================================================
; MAIN
; ============================================================
MAIN:
    LEA DX, MSG_TITLE
    MOV AH, 09h
    INT 21h

    ; [1] Версия DOS ------------------------------------------
    LEA DX, MSG_DOSVER
    MOV AH, 09h
    INT 21h
    MOV AH, 30h
    INT 21h
    PUSH AX
    CALL PRINT_AL_HEX
    MOV DL, '.'
    MOV AH, 02h
    INT 21h
    POP AX
    XCHG AL, AH
    CALL PRINT_AL_HEX
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [2] Размер низкой памяти --------------------------------
    LEA DX, MSG_MEMORY
    MOV AH, 09h
    INT 21h
    INT 12h
    CALL PRINT_AX_DEC
    LEA DX, MSG_KB
    MOV AH, 09h
    INT 21h
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [3] Текущий диск ----------------------------------------
    LEA DX, MSG_DRIVE
    MOV AH, 09h
    INT 21h
    MOV AH, 19h
    INT 21h
    ADD AL, 'A'
    MOV DL, AL
    MOV AH, 02h
    INT 21h
    MOV DL, ':'
    MOV AH, 02h
    INT 21h
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [4] Текущий каталог (ASCIZ!) -----------------------------
    LEA DX, MSG_DIR
    MOV AH, 09h
    INT 21h
    MOV AH, 47h
    XOR DL, DL
    LEA SI, DIR_BUF
    INT 21h
    JC T4_FAIL
    LEA DX, DIR_BUF
    CALL PRINT_ASCIZ
    JMP T4_END
T4_FAIL:
    LEA DX, MSG_FAIL
    MOV AH, 09h
    INT 21h
T4_END:
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [5] Телетайп INT 10h / AH=0Eh ----------------------------
    LEA DX, MSG_TTY
    MOV AH, 09h
    INT 21h
    LEA SI, TTY_STR
T5_LOOP:
    LODSB
    OR AL, AL
    JZ T5_END
    MOV AH, 0Eh
    INT 10h
    JMP T5_LOOP
T5_END:
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [6] Backspace внутри строки AH=09h -----------------------
    LEA DX, MSG_BS
    MOV AH, 09h
    INT 21h
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [7] Прямая запись в B800h: "HI" в (0,75) ------------------
    LEA DX, MSG_DIRECT
    MOV AH, 09h
    INT 21h
    MOV AX, 0B800h
    MOV ES, AX
    MOV DI, 75 * 2
    MOV WORD PTR ES:[DI], 0A48h     ; 'H', attr 0Ah
    MOV WORD PTR ES:[DI+2], 0A49h   ; 'I', attr 0Ah
    LEA DX, MSG_OK
    MOV AH, 09h
    INT 21h

    ; [8] Чтение порта 3DAh ------------------------------------
    LEA DX, MSG_PORT
    MOV AH, 09h
    INT 21h
    MOV DX, 03DAh
    IN AL, DX
    CALL PRINT_AL_HEX
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [9] Текущий видеорежим -----------------------------------
    LEA DX, MSG_VIDEO
    MOV AH, 09h
    INT 21h
    MOV AH, 0Fh
    INT 10h
    CALL PRINT_AL_HEX
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [10] Статус A20 ------------------------------------------
    LEA DX, MSG_A20
    MOV AH, 09h
    INT 21h
    MOV AX, 2400h
    INT 15h
    JC T10_FAIL
    OR AL, AL
    JZ T10_OFF
    LEA DX, MSG_A20_ON
    MOV AH, 09h
    INT 21h
    JMP T10_END
T10_OFF:
    LEA DX, MSG_A20_OFF
    MOV AH, 09h
    INT 21h
    JMP T10_END
T10_FAIL:
    LEA DX, MSG_FAIL
    MOV AH, 09h
    INT 21h
T10_END:
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [11] Проверка DPMI ---------------------------------------
    LEA DX, MSG_DPMI
    MOV AH, 09h
    INT 21h
    MOV AX, 1687h
    INT 2Fh
    JC T11_NOT
    LEA DX, MSG_DPMI_Y
    MOV AH, 09h
    INT 21h
    JMP T11_END
T11_NOT:
    LEA DX, MSG_DPMI_N
    MOV AH, 09h
    INT 21h
T11_END:
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [12] FindFirst + имя через PRINT_ASCIZ -------------------
    LEA DX, MSG_FIND
    MOV AH, 09h
    INT 21h
    MOV AH, 1Ah                 ; Set DTA
    LEA DX, DTA_BUF
    INT 21h
    MOV AH, 4Eh                 ; FindFirst
    XOR CX, CX
    LEA DX, FIND_MASK
    INT 21h
    JC T12_FAIL
    LEA DX, DTA_BUF + 30        ; имя файла в DTA+1Eh
    CALL PRINT_ASCIZ
    JMP T12_END
T12_FAIL:
    LEA DX, MSG_FAIL
    MOV AH, 09h
    INT 21h
T12_END:
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [13] Запись в CON через файловый хендл --------------------
    LEA DX, MSG_CON
    MOV AH, 09h
    INT 21h
    MOV AH, 3Dh
    MOV AL, 2                   ; read/write
    LEA DX, CON_NAME
    INT 21h
    JC T13_FAIL
    MOV BX, AX
    LEA DX, CON_MSG
    MOV CX, CON_MSG_LEN
    MOV AH, 40h
    INT 21h
    MOV AH, 3Eh
    INT 21h
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h
    JMP T13_END
T13_FAIL:
    LEA DX, MSG_FAIL
    MOV AH, 09h
    INT 21h
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h
T13_END:

    ; [14] IOCTL: информация об устройстве (stdin) --------------
    LEA DX, MSG_IOCTL
    MOV AH, 09h
    INT 21h
    MOV AH, 44h
    XOR AL, AL
    XOR BX, BX
    INT 21h
    JC T14_FAIL
    TEST DX, 8000h
    JZ T14_NOTDEV
    LEA DX, MSG_DEV_Y
    MOV AH, 09h
    INT 21h
    JMP T14_END
T14_NOTDEV:
    LEA DX, MSG_DEV_N
    MOV AH, 09h
    INT 21h
    JMP T14_END
T14_FAIL:
    LEA DX, MSG_FAIL
    MOV AH, 09h
    INT 21h
T14_END:
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [15] Неблокирующая проверка клавиши -----------------------
    LEA DX, MSG_KEY1
    MOV AH, 09h
    INT 21h
    MOV AH, 01h
    INT 16h
    JZ T15_NOKEY
    LEA DX, MSG_KEY_YES
    MOV AH, 09h
    INT 21h
    MOV AH, 00h                 ; забираем клавишу
    INT 16h
    JMP T15_END
T15_NOKEY:
    LEA DX, MSG_KEY_NO
    MOV AH, 09h
    INT 21h
T15_END:
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [16] Прямой ввод/вывод консоли AH=06h ---------------------
    LEA DX, MSG_KEY6
    MOV AH, 09h
    INT 21h
    MOV DL, 'Z'
    MOV AH, 06h
    INT 21h
    MOV DL, 0FFh
    MOV AH, 06h
    INT 21h
    JZ T16_NOCHAR
    MOV DL, AL
    MOV AH, 02h
    INT 21h
    JMP T16_END
T16_NOCHAR:
    LEA DX, MSG_KEY_NO
    MOV AH, 09h
    INT 21h
T16_END:
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [17] Блокирующее ожидание клавиши (suspend) ---------------
    LEA DX, MSG_KEYWAIT
    MOV AH, 09h
    INT 21h
    MOV AH, 00h
    INT 16h
    PUSH AX
    LEA DX, MSG_GOTKEY
    MOV AH, 09h
    INT 21h
    POP AX
    OR AL, AL
    JZ T17_EXT
    MOV DL, AL
    MOV AH, 02h
    INT 21h
    JMP T17_END
T17_EXT:
    MOV DL, '['
    MOV AH, 02h
    INT 21h
    MOV AL, AH
    CALL PRINT_AL_HEX
    MOV DL, ']'
    MOV AH, 02h
    INT 21h
T17_END:
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [18] Буферизованный ввод строки AH=0Ah --------------------
    LEA DX, MSG_BUFIN
    MOV AH, 09h
    INT 21h
    LEA DX, INPUT_BUF
    MOV AH, 0Ah
    INT 21h
    LEA SI, INPUT_BUF + 1
    LODSB                       ; AL = кол-во символов
    MOV CL, AL
    XOR CH, CH
    JCXZ T18_END
T18_LOOP:
    LODSB
    MOV DL, AL
    MOV AH, 02h
    INT 21h
    LOOP T18_LOOP
T18_END:
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [19] Mode 13h: заливка + квадрат (без MUL в цикле!) -------
    LEA DX, MSG_MODE13
    MOV AH, 09h
    INT 21h
    MOV AH, 00h
    MOV AL, 13h
    INT 10h
    ; заливка синим (цвет 1)
    MOV AX, 0A000h
    MOV ES, AX
    XOR DI, DI
    MOV AL, 1
    MOV CX, 200
T19_FILL:
    PUSH CX
    MOV CX, 320
    REP STOSB
    POP CX
    LOOP T19_FILL
    ; красный квадрат (100,50)-(199,99): MUL только один раз
    MOV AX, 50
    MOV SI, 320
    MUL SI                      ; AX = 50*320, DX больше не нужен
    ADD AX, 100
    MOV DI, AX
    MOV BP, 50                  ; BP = номер строки (MUL его не тронет)
T19_ROW:
    MOV CX, 100
    MOV BX, DI                  ; запомнили начало строки
T19_COL:
    MOV BYTE PTR ES:[DI], 4     ; красный
    INC DI
    LOOP T19_COL
    MOV DI, BX
    ADD DI, 320                 ; следующая строка
    INC BP
    CMP BP, 100
    JL T19_ROW
    ; ждём клавишу и возвращаемся в текстовый режим
    MOV AH, 00h
    INT 16h
    MOV AH, 00h
    MOV AL, 03h
    INT 10h

    ; [20] Чтение из CON через файловый хендл -------------------
    LEA DX, MSG_CONREAD
    MOV AH, 09h
    INT 21h
    MOV AH, 3Dh
    XOR AL, AL                  ; read-only
    LEA DX, CON_NAME
    INT 21h
    JC T20_FAIL
    MOV BX, AX
    LEA DX, READ_BUF
    MOV CX, 1
    MOV AH, 3Fh
    INT 21h
    MOV DL, READ_BUF
    MOV AH, 02h
    INT 21h
    MOV AH, 3Eh
    INT 21h
    JMP T20_END
T20_FAIL:
    LEA DX, MSG_FAIL
    MOV AH, 09h
    INT 21h
T20_END:
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [21] Курсор: установить (10,20) через AH=02, прочитать AH=03
    LEA DX, MSG_CURSOR
    MOV AH, 09h
    INT 21h
    MOV AH, 02h
    MOV DH, 10
    MOV DL, 20
    MOV BH, 0
    INT 10h
    MOV AH, 03h
    MOV BH, 0
    INT 10h
    MOV AL, DH
    CALL PRINT_AL_HEX
    MOV AL, DL
    CALL PRINT_AL_HEX
    ; вернуть курсор в конец вывода: ставим на строку 24, кол 0
    MOV AH, 02h
    MOV DH, 24
    MOV DL, 0
    INT 10h
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [22] AH=09: записать 'Z' с атрибутом 4Ch 5 раз, прочитать AH=08
    LEA DX, MSG_CELL
    MOV AH, 09h
    INT 21h
    MOV AH, 09h
    MOV AL, 'Z'
    MOV BL, 4Ch
    MOV CX, 5
    MOV BH, 0
    INT 10h
    MOV AH, 08h
    MOV BH, 0
    INT 10h
    CALL PRINT_AL_HEX      ; символ 'Z' = 5A
    ; курсор на новую строку
    MOV AH, 02h
    MOV DH, 24
    MOV DL, 0
    INT 10h
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [23] AH=06: очистить регион (20,0)-(24,79) атрибутом 07h
    LEA DX, MSG_SCROLL
    MOV AH, 09h
    INT 21h
    MOV AH, 06h
    MOV AL, 0          ; 0 = очистить весь регион
    MOV BH, 07h
    MOV CX, 2000h      ; CH=20, CL=00
    MOV DX, 184Fh      ; DH=24, DL=79
    INT 10h
    LEA DX, MSG_OK
    MOV AH, 09h
    INT 21h

        ; [24] Mode 12h: планарная запись через Set/Reset (цвет 4 = красный)
    LEA DX, MSG_MODE12
    MOV AH, 09h
    INT 21h
    MOV AH, 00h
    MOV AL, 12h
    INT 10h
    ; GRC reg 1 (Enable Set/Reset) = 0x0F
    MOV DX, 03CEh
    MOV AL, 01h
    OUT DX, AL
    MOV DX, 03CFh
    MOV AL, 0Fh
    OUT DX, AL
    ; GRC reg 0 (Set/Reset) = 0x04 (красный)
    MOV DX, 03CEh
    MOV AL, 00h
    OUT DX, AL
    MOV DX, 03CFh
    MOV AL, 04h
    OUT DX, AL
    ; Блок 80 строк x 160 пикселей: y=100..179, байты 10..29
    MOV AX, 0A000h
    MOV ES, AX
    MOV DI, 100 * 80 + 10
    MOV CX, 80
T24_ROW:
    PUSH CX
    MOV CX, 20
    MOV AL, 00h          ; значение не важно: цвет даёт Set/Reset
    REP STOSB
    ADD DI, 60           ; до начала следующей строки (80-20)
    POP CX
    LOOP T24_ROW
    MOV AH, 00h
    INT 16h              ; ждём клавишу
    MOV AH, 00h
    MOV AL, 03h
    INT 10h

        ; [25] CRTC Start Address: пишем "PAGE2" в страницу B800:1000 и флипаем
    LEA DX, MSG_PAGE
    MOV AH, 09h
    INT 21h
    MOV AX, 0B800h
    MOV ES, AX
    MOV DI, 1000h
    LEA SI, PAGE2_STR
T25_LOOP:
    LODSB
    OR AL, AL
    JZ T25_SET
    MOV AH, 0Ah          ; атрибут: светло-зелёный на чёрном
    STOSW
    JMP T25_LOOP
T25_SET:
    MOV DX, 03D4h        ; индекс 0x0C = Start Address High
    MOV AL, 0Ch
    OUT DX, AL
    MOV DX, 03D5h
    MOV AL, 10h          ; 0x1000 байт
    OUT DX, AL
    MOV DX, 03D4h        ; индекс 0x0D = Start Address Low
    MOV AL, 0Dh
    OUT DX, AL
    MOV DX, 03D5h
    XOR AL, AL
    OUT DX, AL
    MOV AH, 00h          ; ждём клавишу: на экране должна быть страница 2
    INT 16h
    ; возврат на страницу 0
    MOV DX, 03D4h
    MOV AL, 0Ch
    OUT DX, AL
    MOV DX, 03D5h
    XOR AL, AL
    OUT DX, AL
    LEA DX, MSG_OK
    MOV AH, 09h
    INT 21h

        ; [26] Attribute Controller: палитровый регистр 1 -> DAC 4 (красный)
    LEA DX, MSG_AC
    MOV AH, 09h
    INT 21h
    MOV DX, 03DAh
    IN  AL, DX          ; сброс flip-flop: следующая запись 3C0 = индекс
    MOV DX, 03C0h
    MOV AL, 01h
    OUT DX, AL          ; индекс = палитровый регистр 1
    MOV AL, 04h
    OUT DX, AL          ; данные = DAC 4 (красный)
    ; символ с атрибутом 0x01 должен стать красным
    MOV AH, 09h
    MOV AL, 'A'
    MOV BL, 01h
    MOV CX, 1
    MOV BH, 0
    INT 10h
    ; read-back регистра через 0x3C1
    MOV DX, 03DAh
    IN  AL, DX
    MOV DX, 03C0h
    MOV AL, 01h
    OUT DX, AL
    MOV DX, 03C1h
    IN  AL, DX          ; ожидаем 04
    CALL PRINT_AL_HEX
    ; восстановим регистр 1 = 01h
    MOV DX, 03DAh
    IN  AL, DX
    MOV DX, 03C0h
    MOV AL, 01h
    OUT DX, AL
    MOV AL, 01h
    OUT DX, AL
    MOV AH, 02h         ; курсор на новую строку
    MOV DH, 24
    MOV DL, 0
    INT 10h
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

        ; [27] VBE 640x480x256 windowed: 5 цветных полос через bank switching
    LEA DX, MSG_VBE
    MOV AH, 09h
    INT 21h
    MOV AX, 4F00h
    PUSH DS
    POP ES
    LEA DI, VBE_BUF
    INT 10h
    CMP AX, 004Fh
    JNE T27_FAIL
    CMP WORD PTR ES:[DI], 4556h     ; 'V','E'
    JNE T27_FAIL
    CMP WORD PTR ES:[DI+2], 4153h   ; 'S','A'
    JNE T27_FAIL
    MOV AX, 4F02h
    MOV BX, 0101h                   ; windowed
    INT 10h
    CMP AX, 004Fh
    JNE T27_FAIL
    XOR BP, BP                      ; BP = номер банка 0..4
T27_BANK:
    MOV AX, 4F05h                   ; set bank BX
    MOV BX, BP
    INT 10h
    MOV AX, 0A000h
    MOV ES, AX
    XOR DI, DI
    MOV AX, BP
    ADD AL, 1                       ; цвет = банк+1
    MOV CX, 8000h                   ; ✅ 32768 байт
    REP STOSB
    MOV CX, 8000h                   ; ✅ ещё 32768 => весь банк 64 КБ
    REP STOSB
    INC BP
    CMP BP, 5
    JL T27_BANK
    ; read-back: банк 2, первый байт должен быть 03
    MOV AX, 4F05h
    MOV BX, 2
    INT 10h
    MOV AX, 0A000h
    MOV ES, AX
    MOV AL, ES:[0]
    CALL PRINT_AL_HEX               ; ожид. 03
    MOV AH, 00h                     ; смотрим картинку
    INT 16h
    MOV AX, 0003h
    INT 10h
    JMP T27_DONE
T27_FAIL:
    LEA DX, MSG_VBE_FAIL
    MOV AH, 09h
    INT 21h
T27_DONE:

    ; Финиш ----------------------------------------------------
    LEA DX, MSG_DONE
    MOV AH, 09h
    INT 21h
    LEA DX, MSG_PAUSE        ; ✅ НОВОЕ: ждём клавишу перед выходом
    MOV AH, 09h
    INT 21h
    MOV AH, 00h
    INT 16h
    MOV AH, 4Ch
    INT 21h

; ============================================================
; ПОДПРОГРАММЫ
; ============================================================

; Печать AL как двух hex-цифр (портит BX)
PRINT_AL_HEX PROC NEAR
    PUSH AX
    MOV BL, AL
    MOV CL, 4
    SHR AL, CL
    CMP AL, 10
    JL PAH_HD
    ADD AL, 'A' - 10
    JMP PAH_HP
PAH_HD:
    ADD AL, '0'
PAH_HP:
    MOV DL, AL
    MOV AH, 02h
    INT 21h
    MOV AL, BL
    AND AL, 0Fh
    CMP AL, 10
    JL PAH_LD
    ADD AL, 'A' - 10
    JMP PAH_LP
PAH_LD:
    ADD AL, '0'
PAH_LP:
    MOV DL, AL
    MOV AH, 02h
    INT 21h
    POP AX
    RET
PRINT_AL_HEX ENDP

; Печать AX как десятичного числа
PRINT_AX_DEC PROC NEAR
    PUSH AX
    PUSH BX
    PUSH CX
    PUSH DX
    MOV BX, 10
    XOR CX, CX
PAD_LOOP:
    XOR DX, DX
    DIV BX
    ADD DL, '0'
    PUSH DX
    INC CX
    OR AX, AX
    JNZ PAD_LOOP
PAD_SHOW:
    POP DX
    MOV AH, 02h
    INT 21h
    LOOP PAD_SHOW
    POP DX
    POP CX
    POP BX
    POP AX
    RET
PRINT_AX_DEC ENDP

; Печать ASCIZ-строки по DS:DX посимвольно через AH=02h.
; ЕДИНСТВЕННЫЙ правильный способ печатать буферы, завершённые NUL
; (DIR_BUF, имя из DTA). AH=09h для них использовать НЕЛЬЗЯ.
PRINT_ASCIZ PROC NEAR
    PUSH AX
    PUSH SI
    MOV SI, DX
PAS_LOOP:
    LODSB
    OR AL, AL
    JZ PAS_END
    MOV DL, AL
    MOV AH, 02h
    INT 21h
    JMP PAS_LOOP
PAS_END:
    POP SI
    POP AX
    RET
PRINT_ASCIZ ENDP

END START