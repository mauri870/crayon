; vadd.asm: add scalar 1 to each element of a 4-element vector in memory.
;
; The input array {10, 20, 30, 40} is written by a subroutine (fill_array)
; to demonstrate ret/jb call-and-return.
;
; A0 = 50 (word address used as vector base)
; V0 = {10, 20, 30, 40} loaded from words 50-53
; V1 = 1 + V0 = {11, 21, 31, 41}
; S1-S4 receive extracted elements for verification
;
; Expect: S1=11 S2=21 S3=31 S4=41 cycles=39

    ai a0, 50           ; A0 = 50 (base address)
    ai a1, 4            ; A1 = 4
    setvl a1            ; VL = 4

    ret fill_array      ; fill mem[A0+0..3] = {10, 20, 30, 40}

    vload v0, a0        ; V0 = mem[A0 + n] for n=0..3

    si s0, 1            ; S0 = 1
    vadd v1, s0, v0     ; V1 = S0 + V0

    ai a2, 0
    vget s1, v1, a2     ; S1 = V1[0] = 11
    ai a2, 1
    vget s2, v1, a2     ; S2 = V1[1] = 21
    ai a2, 2
    vget s3, v1, a2     ; S3 = V1[2] = 31
    ai a2, 3
    vget s4, v1, a2     ; S4 = V1[3] = 41

    exit

; fill_array: store {10, 20, 30, 40} to mem[A0+0..3]
; Clobbers: S5
fill_array:
    si s5, 10
    stores s5, a0, 0    ; mem[A0+0] = 10
    si s5, 20
    stores s5, a0, 1    ; mem[A0+1] = 20
    si s5, 30
    stores s5, a0, 2    ; mem[A0+2] = 30
    si s5, 40
    stores s5, a0, 3    ; mem[A0+3] = 40
    jb 0                ; return
