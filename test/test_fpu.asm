; ============================================================
; FPUENC.ASM
;
; TASM DOS COM probe for x87 instruction encodings.
;
; Build:
;   tasm /l /zi FPUENC.ASM
;   tlink /t FPUENC.OBJ,FPUENC.COM
;
; Send back:
;   1. FPUENC.LST
;   2. Ghidra disassembly around start
;
; IMPORTANT:
;   This file intentionally does NOT rely on handwritten opcode
;   tables. TASM is used as the encoding authority.
; ============================================================

.model tiny
.386
.387

.code
org 100h

start:
    ; Marker: control block
    mov ax, 0A000h

    fwait
    fninit
    finit
    fnclex
    fclex

    fnstsw ax
    fstsw  ax

    fnstcw word ptr [cw_save]
    fstcw  word ptr [cw_save]
    fldcw  word ptr [cw_default]

    ; ------------------------------------------------------------
    ; Marker: constants
    ; ------------------------------------------------------------
    mov ax, 0A001h

    fninit
    fld1
    fldl2t
    fldl2e
    fldpi
    fldlg2
    fldln2
    fldz
    fninit

    ; ------------------------------------------------------------
    ; Marker: register stack transfers
    ; ------------------------------------------------------------
    mov ax, 0A002h

    fninit
    fld1
    fldpi
    fldz

    fxch st(1)
    fxch st(2)

    fst  st(0)
    fst  st(1)

    fstp st(0)
    fstp st(0)
    fstp st(0)

    fninit

    ; ------------------------------------------------------------
    ; Marker: unary/basic arithmetic-state instructions
    ; ------------------------------------------------------------
    mov ax, 0A003h

    fninit
    fld1
    fchs
    fabs
    ftst
    fxam
    fsqrt

    fld1
    frndint

    fld1
    fldpi
    fscale

    fincstp
    fdecstp

    fninit

    ; ------------------------------------------------------------
    ; Marker: compare instructions
    ; ------------------------------------------------------------
    mov ax, 0A004h

    fninit
    fld1
    fldpi

    fcom
    fcomp

    fld1
    fldpi
    fcompp

    fninit
    fld1
    fldpi
    fucom st(1)
    fucomp st(1)

    fld1
    fldpi
    fucompp

    fninit

    ; ------------------------------------------------------------
    ; Marker: no-operand arithmetic forms
    ; ------------------------------------------------------------
    mov ax, 0A005h

    fninit
    fld1
    fldpi
    fadd
    fninit

    fld1
    fldpi
    faddp
    fninit

    fld1
    fldpi
    fmul
    fninit

    fld1
    fldpi
    fmulp
    fninit

    fld1
    fldpi
    fsub
    fninit

    fld1
    fldpi
    fsubp
    fninit

    fld1
    fldpi
    fsubr
    fninit

    fld1
    fldpi
    fsubrp
    fninit

    fld1
    fldpi
    fdiv
    fninit

    fld1
    fldpi
    fdivp
    fninit

    fld1
    fldpi
    fdivr
    fninit

    fld1
    fldpi
    fdivrp
    fninit

    ; ------------------------------------------------------------
    ; Marker: m32real load/store
    ; ------------------------------------------------------------
    mov ax, 0A006h

    fninit
    fld  dword ptr [f32_one]
    fadd dword ptr [f32_two]
    fst  dword ptr [f32_out]
    fstp dword ptr [f32_out]

    fld  dword ptr [f32_one]
    fstp dword ptr [f32_out]

    ; ------------------------------------------------------------
    ; Marker: m64real load/store
    ; ------------------------------------------------------------
    mov ax, 0A007h

    fninit
    fld  qword ptr [f64_one]
    fadd qword ptr [f64_two]
    fst  qword ptr [f64_out]
    fstp qword ptr [f64_out]

    fld  qword ptr [f64_one]
    fstp qword ptr [f64_out]

    ; ------------------------------------------------------------
    ; Marker: m80real load/store
    ; ------------------------------------------------------------
    mov ax, 0A008h

    fninit
    fld  tbyte ptr [f80_one]
    fst  tbyte ptr [f80_out]
    fstp tbyte ptr [f80_out]

    fld  tbyte ptr [f80_two]
    fstp tbyte ptr [f80_out]

    ; ------------------------------------------------------------
    ; Marker: integer m16/m32/m64
    ; ------------------------------------------------------------
    mov ax, 0A009h

    fninit
    fild  word ptr [i16_one]
    fist  word ptr [i16_out]
    fistp word ptr [i16_out]

    fninit
    fild  dword ptr [i32_one]
    fist  dword ptr [i32_out]
    fistp dword ptr [i32_out]

    fninit
    fild  qword ptr [i64_one]
    fistp qword ptr [i64_out]

    ; ------------------------------------------------------------
    ; Marker: integer arithmetic memory forms
    ; ------------------------------------------------------------
    mov ax, 0A00Ah

    fninit
    fldz
    fiadd word ptr [i16_one]
    fiadd dword ptr [i32_one]
    fimul word ptr [i16_one]
    fimul dword ptr [i32_one]
    fisub word ptr [i16_one]
    fisub dword ptr [i32_one]
    fisubr word ptr [i16_one]
    fisubr dword ptr [i32_one]
    fidiv word ptr [i16_one]
    fidiv dword ptr [i32_one]
    fidivr word ptr [i16_one]
    fidivr dword ptr [i32_one]
    fninit

    ; ------------------------------------------------------------
    ; Marker: integer compare memory forms
    ; ------------------------------------------------------------
    mov ax, 0A00Bh

    fninit
    fld1
    ficom  word ptr [i16_one]
    ficomp word ptr [i16_one]

    fld1
    ficom  dword ptr [i32_one]
    ficomp dword ptr [i32_one]

    fninit

    ; ------------------------------------------------------------
    ; Marker: BCD
    ; ------------------------------------------------------------
    mov ax, 0A00Ch

    fninit
    fbld  tbyte ptr [bcd_one]
    fbstp tbyte ptr [bcd_out]

    fbld  tbyte ptr [bcd_1234]
    fbstp tbyte ptr [bcd_out]

    fninit

    ; ------------------------------------------------------------
    ; Marker: environment/save/restore
    ; ------------------------------------------------------------
    mov ax, 0A00Dh

    fninit
    fnstenv [env_buf]
    fldenv  [env_buf]

    fnsave  [save_buf]
    frstor  [save_buf]

    fninit

    ; Exit DOS COM program
    mov ax, 4C00h
    int 21h

; ============================================================
; Data
; ============================================================

align 2

cw_default  dw 037Fh
cw_save     dw 0000h
sw_save     dw 0000h

; IEEE754 float32:
; 1.0 = 0x3f800000
; 2.0 = 0x40000000
f32_one     dd 03F800000h
f32_two     dd 040000000h
f32_out     dd 000000000h

; IEEE754 float64:
; 1.0 = 0x3ff0000000000000
; 2.0 = 0x4000000000000000
f64_one     dq 03FF0000000000000h
f64_two     dq 0400000000000000h
f64_out     dq 0000000000000000h

; x87 80-bit:
; 1.0:
;   mantissa = 8000000000000000h
;   sign/exp = 3FFFh
;
; little-endian bytes:
;   00 00 00 00 00 00 00 80 FF 3F
f80_one:
    db 000h, 000h, 000h, 000h, 000h, 000h, 000h, 080h, 0FFh, 03Fh

; 2.0:
;   mantissa = 8000000000000000h
;   sign/exp = 4000h
f80_two:
    db 000h, 000h, 000h, 000h, 000h, 000h, 000h, 080h, 000h, 040h

f80_out:
    db 10 dup(0)

i16_one     dw 0001h
i16_out     dw 0000h

i32_one     dd 00000001h
i32_out     dd 00000000h

i64_one     dq 0000000000000001h
i64_out     dq 0000000000000000h

; Packed BCD +1:
; digits: ...000001, sign byte 00
bcd_one:
    db 001h, 000h, 000h, 000h, 000h, 000h, 000h, 000h, 000h, 000h

; Packed BCD +1234:
; low digits first:
;   byte0 = 34h, byte1 = 12h
bcd_1234:
    db 034h, 012h, 000h, 000h, 000h, 000h, 000h, 000h, 000h, 000h

bcd_out:
    db 10 dup(0)

; Enough space for FNSTENV/FLDENV and FNSAVE/FRSTOR.
; 16-bit/32-bit protected/real environments differ in size.
; 128 bytes is enough for our probe.
env_buf:
    db 128 dup(0)

save_buf:
    db 128 dup(0)

end start