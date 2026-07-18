; fadd.asm: add two floating point constants and verify the result.
;
; Build 1.0 and 2.0 from integer parts using shifts, then add them.
; 1.0 = 0x4001_8000_0000_0000  (exp=0x4001, coeff bit47=1)
; 2.0 = 0x4002_8000_0000_0000  (exp=0x4002, coeff bit47=1)
; 1.0 + 2.0 = 3.0 = 0x4002_C000_0000_0000
;
; Expect: S4=0x4002C00000000000

    ; S2 = 2^47 = 0x0000_8000_0000_0000  (shared coefficient MSB)
    si 2, 1
    sshl 2, 47

    ; S1 = 1.0: place exp 0x4001 in bits 62:48, then OR coefficient
    si 1, 16385       ; S1 = 0x4001
    sshl 1, 48        ; S1 = 0x4001_0000_0000_0000
    sadd 1, 1, 2      ; S1 = 0x4001_8000_0000_0000  (1.0)

    ; S3 = 2.0: same structure, exp 0x4002
    si 3, 16386       ; S3 = 0x4002
    sshl 3, 48        ; S3 = 0x4002_0000_0000_0000
    sadd 3, 3, 2      ; S3 = 0x4002_8000_0000_0000  (2.0)

    ; S4 = 1.0 + 2.0 = 3.0
    fadd 4, 1, 3

    exit
