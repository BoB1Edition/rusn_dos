bits 16
org 0x7C00

mov si, msg
mov ah, 0x0E
print:
    lodsb
    test al, al
    jz halt
    int 0x10
    jmp print
halt:
    cli
    hlt
msg db 'Hello, World!', 0

times 510 - ($ - $$) db 0
dw 0xAA55