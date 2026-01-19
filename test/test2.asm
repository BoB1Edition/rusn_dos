SECTION .text
    org 0x100
	mov ah, 0x9
	mov dx, hello
	int 0x21
    mov esi, [buffer]
    call f1
    ;mov dx, buffer
	;int 0x21
    ;
	mov ax, 0x4c00		; ah == 0x4c al == 0x00
	int 0x21

    f1:
        mov [esi], al
        add [esi], '0'
        dec esi
        add [esi], 0xd
        dec esi
        add [esi], '$'
    ret


SECTION .data
	hello DB "Hello, world!",0xd,0xa,'$'

SECTION .bss
    buffer resb 1024