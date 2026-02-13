.MODEL small
.STACK 100h
.DATA
    headX   dw 160      ; X = 0..319
    headY   dw 100      ; Y = 0..199
    dirX    dw 1        ; Направление по X: -1, 0, +1
    dirY    dw 0        ; Направление по Y: -1, 0, +1
    counter db 0        ; Счётчик для смены направления

.CODE
start:
    ; === Инициализация: вход в режим 13h ===
    MOV AH, 0
    MOV AL, 13h         ; Режим 13h: 320x200, 256 цветов
    INT 10h

main_loop:
    ; === Заливка фона синим цветом ===
    MOV AX, 0A000h      ; Сегмент видеопамяти
    MOV ES, AX
    XOR DI, DI          ; DI = 0 (начало видеопамяти)
    MOV CX, 32000       ; 320*200/2 = 32000 слов
    MOV AL, 1           ; Синий цвет (палитра VGA)
    MOV AH, AL          ; AX = 0x0101
    REP STOSW           ; Заполнение экрана

    ; === Рисуем голову змеи (белая точка) ===
    MOV AX, headY
    MOV BX, AX
    SHL BX, 8           ; BX = headY * 256
    SHL AX, 6           ; AX = headY * 64
    ADD BX, AX          ; BX = headY * 320
    MOV AX, headX
    ADD BX, AX          ; BX = headY * 320 + headX
    MOV DI, BX
    MOV AL, 15          ; Белый цвет
    STOSB               ; Запись в видеопамять [ES:DI]

    ; === Обновление позиции (автоматическое движение) ===
    MOV AX, dirX
    ADD headX, AX       ; headX += dirX

    MOV AX, dirY
    ADD headY, AX       ; headY += dirY

    ; === Проверка столкновения со стенами ===
    ; Правая стена (X >= 320)
    MOV AX, headX
    CMP AX, 320
    JAE wrap_right
    JMP check_left

wrap_right:
    MOV headX, 0
    JMP update_dir

check_left:
    CMP headX, 0
    JGE check_bottom
    MOV headX, 319
    JMP update_dir

check_bottom:
    MOV AX, headY
    CMP AX, 200
    JAE wrap_bottom
    JMP check_top

wrap_bottom:
    MOV headY, 0
    JMP update_dir

check_top:
    CMP headY, 0
    JGE update_dir
    MOV headY, 199
    JMP update_dir

update_dir:
    ; === Смена направления каждые 40 кадров ===
    INC counter
    CMP counter, 40
    JB skip_dir_change
    MOV counter, 0      ; Сброс счётчика

    ; Циклическая смена направления: вправо → вниз → влево → вверх
    MOV AX, dirX
    CMP AX, 1
    JNE check_down
    MOV dirX, 0
    MOV dirY, 1         ; Теперь вниз
    JMP skip_dir_change

check_down:
    CMP AX, 0
    JNE check_left_dir
    MOV AX, dirY
    CMP AX, 1
    JNE check_left_dir
    MOV dirX, -1
    MOV dirY, 0         ; Теперь влево
    JMP skip_dir_change

check_left_dir:
    CMP dirX, -1
    JNE check_up
    MOV dirX, 0
    MOV dirY, -1        ; Теперь вверх
    JMP skip_dir_change

check_up:
    MOV AX, dirY
    CMP AX, -1
    JNE skip_dir_change
    MOV dirX, 1
    MOV dirY, 0         ; Теперь вправо

skip_dir_change:
    ; === Задержка для контроля скорости ===
    MOV CX, 0FFFh
delay_loop:
    LOOP delay_loop

    ; === Выход после 200 итераций (ИСПРАВЛЕНО: короткий прыжок вперёд) ===
    CMP counter, 200
    JB continue_loop    ; Короткий прыжок ВПЕРЁД (в пределах 127 байт)
    JMP exit_game       ; Длинный прыжок на выход

continue_loop:
    JMP main_loop       ; Безусловный прыжок назад (поддерживает длинные переходы)

exit_game:
    ; === Завершение: возврат в текстовый режим ===
    MOV AH, 0
    MOV AL, 03h         ; Текстовый режим 80x25
    INT 10h

    ; Выход из программы
    MOV AX, 4C00h
    INT 21h

END start
