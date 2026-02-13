.MODEL small
.STACK 100h
.DATA

menuText db "1. Option One",13,10,"2. Option Two",13,10,"3. Exit",0

.CODE
start:
    ; Войти в графический режим 13h (320x200, 256 цветов)
    mov ah, 0
    mov al, 13h  ; видеорежим 13h
    int 10h

    ; Заливка фона синим цветом
    ; цвет: 1 (синий в 256-цветном режиме VGA)
    mov ax, 0A000h
    mov es, ax
    mov di, 0
    mov cx, 320*200/2 ; количество пар байтов для полного экрана
    mov al, 01h       ; цвет синий
    rep stosw

    ; Вывести меню (текст в верхней части экрана)
    ; Для этого используем BIOS funct 0Eh для вывода текста на полосе экрана
    mov si, offset menuText
    call print_text

    ; Ожидание нажатия клавиши для завершения
    mov ah, 0
    int 16h

    ; Вернуться в текстовый режим 03h
    mov ah, 0
    mov al, 03h
    int 10h

    mov ax, 4C00h
    int 21h

; Функция для вывода текста (строчного режима VGA)
; SI указывает на строку текста с управляющими символами (CR,LF)
print_text:
    push ax
    push di
print_char:
    lodsb
    cmp al, 0
    je done
    cmp al, 13
    jne not_cr
    ; Перейти на новую строку (вниз)
    ; Для быстроты используют BIOS видеосервис
    mov ah, 0Eh
    mov bh, 0
    mov bl, 07h
    mov cx, 80  ; примерно ширина строки
    mov dl, 0
    int 10h
    jmp print_char
not_cr:
    cmp al, 10
    jne print_char_char
    jmp print_char
print_char_char:
    ; Выводим символ
    mov ah, 0Eh
    mov bh, 0
    mov bl, 07h
    mov al, al
    int 10h
    jmp print_char
done:
    pop di
    pop ax
    ret

END start