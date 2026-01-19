format MZ
entry main:start

segment data_seg
    hello:  db 'Hello World', 13, 10, '$'

segment stack_seg
    stack_size db 64
stacktop:

segment main
start:
    mov dx, hello        ; Load address of string
    mov ah, 9            ; DOS print string function
    int 0x21             ; Call DOS interrupt
    mov ax, 0x4c00       ; DOS exit function
    int 0x21 

