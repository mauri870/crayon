; fadd.asm: add two floating point constants and verify the result.
;
; Build 1.0 and 2.0 from integer parts using shifts, then add them.
; 1.0 = 0x4001_8000_0000_0000  (exp=0x4001, coeff bit47=1)
; 2.0 = 0x4002_8000_0000_0000  (exp=0x4002, coeff bit47=1)
; 1.0 + 2.0 = 3.0 = 0x4002_C000_0000_0000
;
; Expect: S4=0x4002C00000000000 cycles=14

    ; S2 = 2^47 = 0x0000_8000_0000_0000  (shared coefficient MSB)
    si s2, 1
    sshl s2, 47

    ; S1 = 1.0: place exp 0x4001 in bits 62:48, then OR coefficient
    si s1, 16385       ; S1 = 0x4001
    sshl s1, 48        ; S1 = 0x4001_0000_0000_0000
    sadd s1, s1, s2    ; S1 = 0x4001_8000_0000_0000  (1.0)

    ; S3 = 2.0: same structure, exp 0x4002
    si s3, 16386       ; S3 = 0x4002
    sshl s3, 48        ; S3 = 0x4002_0000_0000_0000
    sadd s3, s3, s2    ; S3 = 0x4002_8000_0000_0000  (2.0)

    ; S4 = 1.0 + 2.0 = 3.0
    fadd s4, s1, s3

    exit
