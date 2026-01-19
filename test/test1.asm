org 0x100
mov dx, msg
mov ah, 09h
int 21h
mov ax, 4C00h
int 21h
msg: db 'Hello, DOS!$'