; vadd.asm: add scalar 1 to each element of a 4-element vector in memory.
;
; A0 = 50 (word address used as vector base)
; V0 = {10, 20, 30, 40} loaded from words 50-53
; V1 = 1 + V0 = {11, 21, 31, 41}
; S1-S4 receive extracted elements for verification
;
; Expect: S1=11 S2=21 S3=31 S4=41

    ai 0, 50          ; A0 = 50 (base address)
    ai 1, 4           ; A1 = 4
    setvl 1           ; VL = 4

    ; store input array to words 50-53
    si 5, 10
    stores 5, 0, 0    ; mem[A0+0] = 10
    si 5, 20
    stores 5, 0, 1    ; mem[A0+1] = 20
    si 5, 30
    stores 5, 0, 2    ; mem[A0+2] = 30
    si 5, 40
    stores 5, 0, 3    ; mem[A0+3] = 40

    vload 0, 0        ; V0 = mem[A0 + n] for n=0..3

    si 0, 1           ; S0 = 1
    vadd 1, 0, 0      ; V1 = S0 + V0

    ai 2, 0
    vget 1, 1, 2      ; S1 = V1[0] = 11
    ai 2, 1
    vget 2, 1, 2      ; S2 = V1[1] = 21
    ai 2, 2
    vget 3, 1, 2      ; S3 = V1[2] = 31
    ai 2, 3
    vget 4, 1, 2      ; S4 = V1[3] = 41

    exit
