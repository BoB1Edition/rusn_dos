SECTION .text
mov eax, text
mov ds, eax
mov dx, ds:text
xor eax, eax
mov ah, 0x9
int 21h
mov cx, 4
mov esi, [buffer]
call loop

mov dx, buffer
mov ah, 0x9
int 21h

xor ah, ah
xor al, al
mov ax, 4C00h
int 21h

loop:
    xor edx, edx        ; Clear EDX for the DIV operation (EAX is 32-bit)
    mov ebx, 10         ; Divisor is 10
    div ebx             ; Divide EAX by 10. Quotient in EAX, remainder in EDX
    add edx, '0'        ; Convert remainder to ASCII digit
    dec esi             ; Move to the previous character position in the buffer
    mov [esi], dl       ; Store the ASCII digit

    test eax, eax       ; Check if the quotient (EAX) is zero
    jnz loop    ; If not zero, continue the loop
    dec esi
    mov [esi], '$'

    ret


SECTION .data
    text db 'How?'
section .bss
    buffer resb 1024