; ============================================================
;  TESTEXEC.ASM — тест INT 21h/AH=4Bh (EXEC) + AH=4Dh
;  Сборка: TASM TESTEXEC.ASM / TLINK /t TESTEXEC.OBJ
;  Ожидание: выполняется TESTCMD.COM с аргументом 7,
;            затем печатается код возврата 07
; ============================================================
.MODEL TINY
.CODE
ORG 100h
START:
    JMP MAIN

MSG_TITLE DB '=== EXEC test ===', 0Dh, 0Ah, '$'
MSG_RUN   DB 'Calling TESTCMD.COM 7 via EXEC...', 0Dh, 0Ah, '$'
MSG_RC    DB 0Dh, 0Ah, 'Child exit code: $'
MSG_FAIL  DB 'EXEC failed!', 0Dh, 0Ah, '$'
MSG_CRLF  DB 0Dh, 0Ah, '$'
PROG      DB 'TESTCMD.COM', 0
TAIL      DB 2, ' 7', 0Dh        ; байт длины, " 7", CR
PB_ENV    DW 0, 0                ; env = 0 (наследовать)
PB_CMD    DW 0, 0
PB_F1     DW 0, 0
PB_F2     DW 0, 0

MAIN:
    LEA DX, MSG_TITLE
    MOV AH, 09h
    INT 21h

    ; Заполняем параметрический блок (сегмент = PSP)
    MOV AX, DS
    MOV PB_CMD, OFFSET TAIL
    MOV PB_CMD+2, AX
    MOV PB_F1, 5Ch
    MOV PB_F1+2, AX
    MOV PB_F2, 6Ch
    MOV PB_F2+2, AX

    LEA DX, MSG_RUN
    MOV AH, 09h
    INT 21h

    MOV AX, 4B00h
    LEA DX, PROG
    LEA BX, PB_ENV
    INT 21h
    JC EXEC_FAIL

    MOV AH, 4Dh
    INT 21h            ; AL = код возврата ребёнка
    PUSH AX
    LEA DX, MSG_RC
    MOV AH, 09h
    INT 21h
    POP AX
    CALL PRINT_AL_HEX  ; ожидаем 07
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h
    JMP DONE

EXEC_FAIL:
    LEA DX, MSG_FAIL
    MOV AH, 09h
    INT 21h

DONE:
    MOV AH, 4Ch
    INT 21h

; Печать AL как двух hex-цифр
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

END START