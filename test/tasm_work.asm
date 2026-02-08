; hello.asm
.MODEL SMALL
.STACK 100h

.DATA
    msg DB 'hello$'   ; строка, концовка $

.CODE
main PROC
    MOV AX, @DATA
    MOV DS, AX

    ; вывод строки
    LEA DX, msg
    MOV AH, 09h
    INT 21h

    ; завершение программы
    MOV AH, 4Ch
    INT 21h
main ENDP

END main