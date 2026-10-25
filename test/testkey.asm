; ============================================================
;  TESTKEY.ASM — тест консольного ввода (Этап 3)
;  Ждёт клавиши (INT 16h/AH=00h), печатает символ и scancode/ascii,
;  Esc — выход.
;  Сборка: TASM TESTKEY.ASM / TLINK /t TESTKEY.OBJ
; ============================================================
.MODEL TINY
.CODE
ORG 100h
START:
    JMP MAIN

MSG_HELLO DB 'TESTKEY: press keys (Esc = exit)', 0Dh, 0Ah, '$'
MSG_GOT   DB '  char/sc-ascii: $'
MSG_CRLF  DB 0Dh, 0Ah, '$'

MAIN:
    LEA DX, MSG_HELLO
    MOV AH, 09h
    INT 21h
KEY_LOOP:
    MOV AH, 00h
    INT 16h              ; AX = scancode:ascii
    CMP AL, 1Bh          ; Esc
    JE KEY_EXIT
    PUSH AX
    MOV DL, AL
    CMP AL, 20h
    JAE GOT_PRINT
    MOV DL, '.'          ; непечатаемое — точкой
GOT_PRINT:
    MOV AH, 02h
    INT 21h
    POP AX
    CALL PRINT_AX_HEX
    LEA DX, MSG_CRLF
    MOV AH, 09h
    INT 21h
    JMP KEY_LOOP
KEY_EXIT:
    MOV AH, 4Ch
    INT 21h

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