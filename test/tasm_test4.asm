.MODEL small
.STACK 100h
.DATA
menuItems db "1. Option One",13,10,"2. Option Two",13,10,"3. Exit",0
menuPointers dw offset menuItems, offset menuItems+17, offset menuItems+34
numItems dw 3
cursorIndex dw 0

; Цвета
BLUE equ 1
HIGHLIGHT equ 4h

.CODE
start:
    ; Войти в графический режим 13h
    mov ah, 0
    mov al, 13h
    int 10h

    ; Заливка синим цветом
    mov ax, 0A000h
    mov es, ax
    xor di, di
    mov cx, 320 * 200 / 2
    mov al, BLUE
    rep stosw

    ; Инициализация мыши
    call init_mouse

    ; Вывод меню
    call draw_menu

    ; Основной цикл обработки
main_loop:
    call check_mouse
    call process_input
    jmp main_loop

; Восстановление режима и завершение
    mov ah, 0
    mov al, 03h
    int 10h
    mov ax, 4C00h
    int 21h

; ======================================
; Инициализация мыши
init_mouse:
    mov ax, 0
    int 33h             ; Проверка мыши
    cmp ax, 0
    je no_mouse
    ; Если есть мышь, включаем
    mov ax, 0
    int 33h
    ret
no_mouse:
    ; Мышь не обнаружена, можно завершить или продолжить
    ret

; ======================================
; Отрисовка меню
draw_menu:
    mov si, offset menuItems
    mov cx, numItems
    mov bx, 0             ; индекс элемента
    mov dx, 20            ; позиция Y для меню

draw_loop:
    push bx
    ; Расчет позиции Y для каждого элемента
    mov di, 0
    mov ah, 0Eh        ; BIOS текст вывод
    call print_string_at

    inc bx
    inc dx
    pop bx
    loop draw_loop
    ret

; ======================================
; Вывод строки по координатам (X=10, Y=ответ ниже)
print_string_at:
    push ax
    push di
    ; Текущий индекс элемента - в BX
    ; Находим указатель к строке
    mov si, offset menuItems
    ; Вычисляем указатель
    mov di, bx
    dec di
    mov di, di
    lea di, [si + di * 17]  ; длина строки + CRLF
    mov si, [di]            ; адрес строки
    ; Перемещаем курсор в координаты X=10, Y=dx
    mov ah, 02h
    mov bh, 0
    mov dl, 10
    mov dh, dx
    int 10h
    ; Выводим строку посимвольно
    ;  - тут можно расширить, пропустить, выводить через BIOS или напрямую
    mov si, [di]
    mov cx, 17
    mov ah, 0Eh
.print_char:
    lodsb
    cmp al, 0
    je .done
    int 10h
    loop .print_char
.done:
    pop di
    pop ax
    ret

; ======================================
; Обработка мыши
check_mouse:
    mov ax, 3     ; Проверка мыши
    int 33h
    cmp ax, 0
    je no_mouse_action
    ; Если мышь есть
    mov ax, 3     ; Получить состояние мыши
    int 33h
    ; AX: ботинки, bx, dx содержат координаты и состояние кнопки
    ; Обработка навигации
    ; Пример анализа координат мыши для изменения cursorIndex
    ; (например, если мышь кликает на меню)
    ; Можно добавить обработку клика или движение
    ; For simplicity, предполагаем просто движение курсора
    ; по Y для выбора пункта
    mov bx, 0
    mov dx, 20
    mov si, offset menuItems
    mov cx, numItems
    ; Проверка по Y-координате мыши
    ; (здесь нужно конкретно считать координаты мыши)
    ; Для простоты пропустим, сейчас — ничего не делаем
    ret
no_mouse_action:
    ret

; ======================================
; Обработка клавиш (например, стрелок)
process_input:
    ; Можно добавить обработку клавиш для перемещения курсора
    ; или ESC для выхода
    mov ah, 0
    int 16h
    cmp al, 27
    je finish
    cmp al, 0dh
    jne continue
    ; ENTER
    ; Обработка выбора
continue:
    ret

finish:
    mov ax, 4C00h
    int 21h

END start