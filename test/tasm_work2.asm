; Ver: 1
.MODEL TINY
.CODE

main PROC
    MOV AX, 0   ; PSP всегда в сегменте 0 для .COM
    MOV ES, AX

    ; Читаем слово из ES:[0x0002]
    MOV AX, WORD PTR ES:[0002h] ; ← ИСПРАВЛЕНО: 0002h + WORD PTR

    ; Выводим значение в шестнадцатеричном виде
    MOV BX, AX
    MOV CX, 4
print_loop:
    ROL BX, 4
    MOV DL, BL
    AND DL, 0Fh
    CMP DL, 10
    JB  print_digit
    ADD DL, 7       ; 'A'-'F'
print_digit:
    ADD DL, '0'     ; '0'-'9'
    MOV AH, 02h     ; DOS функция вывода символа
    INT 21h
    LOOP print_loop

    ; Новая строка
    MOV DL, 13
    MOV AH, 02h
    INT 21h
    MOV DL, 10
    MOV AH, 02h
    INT 21h

    ; Завершение
    MOV AX, 4C00h
    INT 21h
main ENDP
END main