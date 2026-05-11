.MODEL SMALL
.STACK 100h
.DATA
    player_x    DW 150
    obj_x       DW 150
    obj_y       DW 0
    score       DW 0
    misses      DB 0
    speed       DB 2
    score_str   DB 'Score: 00000', 13, 10, '$'
    over_msg    DB 'GAME OVER! Press ESC to exit.', 13, 10, '$'

.CODE
START:
    MOV AX, @DATA
    MOV DS, AX

    ; 1. Установка режима 13h
    MOV AX, 0013h
    INT 10h

MAIN_LOOP:
    ; 2. Очистка экрана
    MOV AX, 0A000h
    MOV ES, AX
    XOR DI, DI
    MOV CX, 32000
    XOR AX, AX
    REP STOSW

    ; 3. Рисуем игрока
    MOV BX, 190
    MOV CX, player_x
    MOV DX, 16
    MOV AL, 9
    CALL DrawLine
    INC BX
    CALL DrawLine

    ; 4. Рисуем объект
    MOV BX, obj_y
    MOV CX, obj_x
    MOV DX, 4
    MOV AL, 15
    CALL DrawLine
    INC BX
    CALL DrawLine
    INC BX
    CALL DrawLine
    INC BX
    CALL DrawLine

    ; 5. Задержка
    MOV CX, 500
.DELAY_LOOP:
    LOOP .DELAY_LOOP

    ; 6. Опрос клавиатуры (ИСПРАВЛЕНО: все JE заменены на JNE+JMP)
    IN AL, 64h
    TEST AL, 01h
    JZ SKIP_INPUT
    IN AL, 60h
    CMP AL, 4Bh          ; Стрелка влево?
    JE MOVE_LEFT
    CMP AL, 4Dh          ; Стрелка вправо?
    JNE CHECK_ESC        ; Если нет -> проверяем дальше
    JMP MOVE_RIGHT       ; Безусловный переход: дальний
CHECK_ESC:
    CMP AL, 01h          ; ESC?
    JNE SKIP_INPUT       ; Если нет -> идем к обработке физики
    JMP GAME_OVER        ; Безусловный переход: дальний

SKIP_INPUT:
    ; 7. Движение объекта
    MOV AL, speed
    CBW
    ADD obj_y, AX
    MOV BX, obj_y
    CMP BX, 200
    JGE DO_MISS          ; <-- Исправлено: короткое смещение
    ; 8. Проверка столкновения
    CMP BX, 190
    JL NO_HIT
    MOV AX, obj_x
    MOV BX, player_x
    SUB AX, BX
    CMP AX, 16
    JGE NO_HIT
    CMP AX, -4
    JL NO_HIT
    INC score
    JMP RESET_OBJ

DO_MISS:
    INC misses
    CMP misses, 3
    JE GAME_OVER

RESET_OBJ:
    MOV obj_y, 0
    MOV obj_x, 160
    MOV AX, score
    AND AX, 7
    JNZ NO_HIT
    INC speed

NO_HIT:
    ; 9. Вывод счёта
    MOV AX, score
    LEA SI, score_str + 7
    CALL NumToStr
    MOV AH, 09h
    LEA DX, score_str
    INT 21h
    JMP MAIN_LOOP

; --- Обработчики ввода ---
MOVE_LEFT:
    MOV AX, player_x
    SUB AX, 5
    CMP AX, 0
    JL PL_ZERO
    MOV player_x, AX
    JMP MAIN_LOOP
PL_ZERO:
    MOV player_x, 0
    JMP MAIN_LOOP

MOVE_RIGHT:
    MOV AX, player_x
    ADD AX, 5
    CMP AX, 305
    JG PR_MAX
    MOV player_x, AX
    JMP MAIN_LOOP
PR_MAX:
    MOV player_x, 305
    JMP MAIN_LOOP

; --- Конец игры ---
GAME_OVER:
    MOV AH, 09h
    LEA DX, over_msg
    INT 21h
WAIT_KEY:
    IN AL, 64h
    TEST AL, 01h
    JZ WAIT_KEY
    IN AL, 60h
    CMP AL, 01h
    JNE WAIT_KEY
    MOV AX, 4C00h
    INT 21h

; --- Вспомогательные процедуры ---
DrawLine: ; BX=Y, CX=X, DX=ширина, AL=цвет
    PUSH BX 
    PUSH CX 
    PUSH DX 
    PUSH AX
    MOV AH, AL
    MOV DI, BX
    SHL DI, 8
    MOV AX, BX
    SHL AX, 6
    ADD DI, AX
    ADD DI, CX
    MOV CX, DX
.DRAW:
    MOV ES:[DI], AH
    INC DI
    LOOP .DRAW
    POP AX 
    POP DX 
    POP CX 
    POP BX
    RET

NumToStr: ; AX=число, SI=указатель на конец строки (5 цифр)
    PUSH BX 
    PUSH CX 
    PUSH DX
    MOV BX, 10000
    MOV CX, 5
.DIGIT_LOOP:
    MOV DL, '0'
    CMP AX, BX
    JL .STORE_DIGIT
.SUB:
    SUB AX, BX
    INC DL
    CMP AX, BX
    JGE .SUB
.STORE_DIGIT:
    MOV [SI], DL
    INC SI
    ; Делим BX на 10
    MOV DX, BX
    MOV BX, 0
    MOV CX, 10
.DIV10:
    CMP DX, CX
    JL .DIV_DONE
    SUB DX, CX
    INC BX
    JMP .DIV10
.DIV_DONE:
    MOV BX, DX
    LOOP .DIGIT_LOOP
    POP DX 
    POP CX 
    POP BX
    RET

END START