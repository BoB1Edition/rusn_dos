; ============================================================
;  TESTCMD.ASM — тест командной строки / PSP / окружения (COM)
;  Ver 2: только безопасные для TASM 4.1 формы операндов
; ============================================================
;  Сборка:  TASM TESTCMD.ASM
;           TLINK /t TESTCMD.OBJ
;  Проверки:
;   [1] байт длины command tail (PSP:80h)
;   [2] hex-дамп символов tail (PSP:81h..)
;   [3] tail как строка (непечатаемое -> '.')
;   [4] FCB1 (PSP:5Ch), 12 байт hex
;   [5] FCB2 (PSP:6Ch), 12 байт hex
;   [6] сегмент окружения (PSP:2Ch) + дамп строк окружения
;   [7] регистры на входе: AX BX CX DX SI DI BP SP
;   [8] если первый аргумент - цифра 0..9, exit code = эта цифра
; ============================================================
.MODEL TINY
.CODE
ORG 100h
START:
    JMP MAIN

MSG_TITLE DB '=== TESTCMD v2: cmdline / PSP / env test ===', 0Dh, 0Ah, '$'
MSG_TLEN  DB '[1] Tail length byte: $'
MSG_THEX  DB '[2] Tail hex: $'
MSG_TSTR  DB '[3] Tail str: $'
MSG_FCB1  DB '[4] FCB1: $'
MSG_FCB2  DB '[5] FCB2: $'
MSG_ENVS  DB '[6] Env segment: $'
MSG_REGS  DB '[7] Entry regs AX BX CX DX SI DI BP SP: $'
MSG_EXIT  DB '[8] Single digit arg -> exit(digit)', 0Dh, 0Ah, '$'
MSG_CRLF  DB 0Dh, 0Ah, '$'
MSG_SPACE DB ' $'

SAVE_AX DW 0
SAVE_BX DW 0
SAVE_CX DW 0
SAVE_DX DW 0
SAVE_SI DW 0
SAVE_DI DW 0
SAVE_BP DW 0
SAVE_SP DW 0

MAIN:
    MOV SAVE_AX, AX
    MOV SAVE_BX, BX
    MOV SAVE_CX, CX
    MOV SAVE_DX, DX
    MOV SAVE_SI, SI
    MOV SAVE_DI, DI
    MOV SAVE_BP, BP
    MOV SAVE_SP, SP

    LEA DX, MSG_TITLE
    MOV AH, 09h
    INT 21h

    ; [1] длина tail (DS = PSP в COM)
    LEA DX, MSG_TLEN
    MOV AH, 09h
    INT 21h
    MOV BX, 80h
    MOV AL, [BX]
    CALL PRINT_AL_HEX
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [2] hex-дамп tail
    LEA DX, MSG_THEX
    MOV AH, 09h
    INT 21h
    MOV BX, 80h
    XOR CH, CH
    MOV CL, [BX]
    MOV SI, 81h
HEX_LOOP:
    OR CL, CL
    JZ HEX_DONE
    MOV AL, [SI]
    PUSH CX
    CALL PRINT_AL_HEX
    POP CX
    LEA DX, MSG_SPACE
    MOV AH, 09h
    INT 21h
    INC SI
    DEC CL
    JMP HEX_LOOP
HEX_DONE:
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [3] tail строкой
    LEA DX, MSG_TSTR
    MOV AH, 09h
    INT 21h
    MOV BX, 80h
    XOR CH, CH
    MOV CL, [BX]
    MOV SI, 81h
STR_LOOP:
    OR CL, CL
    JZ STR_DONE
    MOV AL, [SI]
    CMP AL, 20h
    JAE STR_PUT
    MOV AL, '.'
STR_PUT:
    MOV DL, AL
    MOV AH, 02h
    INT 21h
    INC SI
    DEC CL
    JMP STR_LOOP
STR_DONE:
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [4] FCB1
    LEA DX, MSG_FCB1
    MOV AH, 09h
    INT 21h
    MOV SI, 5Ch
    MOV CL, 12
    CALL PRINT_MEM_HEX
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [5] FCB2
    LEA DX, MSG_FCB2
    MOV AH, 09h
    INT 21h
    MOV SI, 6Ch
    MOV CL, 12
    CALL PRINT_MEM_HEX
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [6] окружение
    LEA DX, MSG_ENVS
    MOV AH, 09h
    INT 21h
    MOV BX, 2Ch
    MOV AX, [BX]
    CALL PRINT_AX_HEX
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h
    MOV ES, AX
    XOR DI, DI
ENV_LOOP:
    MOV AL, ES:[DI]
    OR AL, AL
    JZ ENV_GAP
    CMP AL, 20h
    JAE ENV_PUT
    MOV AL, '.'
ENV_PUT:
    MOV DL, AL
    MOV AH, 02h
    INT 21h
    INC DI
    JMP ENV_LOOP
ENV_GAP:
    INC DI
    MOV AL, ES:[DI]
    OR AL, AL
    JZ ENV_DONE
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h
    JMP ENV_LOOP
ENV_DONE:
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [7] регистры на входе
    LEA DX, MSG_REGS
    MOV AH, 09h
    INT 21h
    MOV AX, SAVE_AX
    CALL PRINT_AX_HEX
    LEA DX, MSG_SPACE
    MOV AH, 09h
    INT 21h
    MOV AX, SAVE_BX
    CALL PRINT_AX_HEX
    LEA DX, MSG_SPACE
    MOV AH, 09h
    INT 21h
    MOV AX, SAVE_CX
    CALL PRINT_AX_HEX
    LEA DX, MSG_SPACE
    MOV AH, 09h
    INT 21h
    MOV AX, SAVE_DX
    CALL PRINT_AX_HEX
    LEA DX, MSG_SPACE
    MOV AH, 09h
    INT 21h
    MOV AX, SAVE_SI
    CALL PRINT_AX_HEX
    LEA DX, MSG_SPACE
    MOV AH, 09h
    INT 21h
    MOV AX, SAVE_DI
    CALL PRINT_AX_HEX
    LEA DX, MSG_SPACE
    MOV AH, 09h
    INT 21h
    MOV AX, SAVE_BP
    CALL PRINT_AX_HEX
    LEA DX, MSG_SPACE
    MOV AH, 09h
    INT 21h
    MOV AX, SAVE_SP
    CALL PRINT_AX_HEX
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h

    ; [8] exit code = цифра-аргумент (учитывает ведущий пробел и его отсутствие)
    LEA DX, MSG_EXIT
    MOV AH, 09h
    INT 21h
    MOV SI, 81h
    MOV AL, [SI]
    CMP AL, 20h
    JNE CHK_DIGIT
    INC SI
    MOV AL, [SI]
CHK_DIGIT:
    CMP AL, '0'
    JB EXIT_ZERO
    CMP AL, '9'
    JA EXIT_ZERO
    SUB AL, '0'
    MOV AH, 4Ch
    INT 21h
EXIT_ZERO:
    XOR AL, AL
    MOV AH, 4Ch
    INT 21h

; ---------- подпрограммы ----------
; PRINT_MEM_HEX: SI = смещение в DS, CL = число байт
PRINT_MEM_HEX PROC NEAR
    PUSH AX
    PUSH CX
    PUSH SI
PMH_LOOP:
    OR CL, CL
    JZ PMH_END
    MOV AL, [SI]
    PUSH CX
    CALL PRINT_AL_HEX
    POP CX
    MOV DL, 20h
    MOV AH, 02h
    INT 21h
    INC SI
    DEC CL
    JMP PMH_LOOP
PMH_END:
    POP SI
    POP CX
    POP AX
    RET
PRINT_MEM_HEX ENDP

; Печать AL как двух hex-цифр (сохраняет AX)
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

; Печать AX как четырёх hex-цифр
PRINT_AX_HEX PROC NEAR
    PUSH AX
    MOV AL, AH
    CALL PRINT_AL_HEX
    POP AX
    CALL PRINT_AL_HEX
    RET
PRINT_AX_HEX ENDP

END START