; // Ver: 1
.MODEL small
.STACK 100h

.DATA
msg1 DB 'Hello, BIOS!', 0Dh, 0Ah, '$'    ; сообщение для вывода

filename DB 'TEST.TXT', 0     ; имя файла
filehandle DW ?             ; дескриптор файла
errorMsg DB 'Error opening file!', 0Dh, 0Ah, '$'

.CODE

main PROC
    ; ----------- Работа с BIOS input/out -----------
    ; Ввод байта с клавиатуры через BIOS
    mov ah, 1               ; чтение символа с клавиатуры, без Эхо
    int 21h
    ; символ в AL, можно вывести обратно
    ; вывод символа на экран через BIOS
    mov ah, 0Eh             ; функция teletype
    mov bh, 0               ; страница, обычно 0
    mov bl, 07h             ; цвет (обычный)
    int 10h                 ; вывод символа

    ; ----------- Вывод сообщения на экран -----------

    lea dx, msg1
    mov ah, 09h            ; вывести строку
    int 21h

    ; ----------- Открытие файла -----------

    lea dx, filename
    mov ah, 3Dh            ; открыть файл
    mov al, 2              ; режим - для записи
    int 21h
    jc file_error          ; если ошибка, прыгнуть
    mov filehandle, ax     ; сохранить дескриптор файла

    ; ----------- Запись данных в файл -----------

    ; пример данных
    mov dx, offset msg1
    mov ah, 40h            ; записать в файл
    mov bx, filehandle
    mov cx, 13             ; длина строки "Hello, BIOS!"
    int 21h

    ; ----------- Закрытие файла -----------

    mov ah, 3Eh
    mov bx, filehandle
    int 21h

    ; ----------- Работа с портами и чтение/запись -----------

    mov dx, 70h          ; порт BIOS data area или любой другой порт
    in al, dx              ; чтение байта
    mov bl, al             ; сохранить
    ; Вывод полученного байта на экран (через BIOS)
    mov ah, 0Eh
    mov al, bl
    int 10h

    ; Запись байта обратно в порт
    mov dx, 70h
    mov al, bl
    out dx, al

    ; завершить программу
    mov ah, 4Ch
    int 21h

file_error:
    ; Обработка ошибки открытия файла
    lea dx, errorMsg
    mov ah, 09h
    int 21h

    mov ah, 4Ch
    int 21h
ENDP main
END main