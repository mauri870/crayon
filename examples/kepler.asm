; kepler.asm: Kepler's Third Law: T² = a³
;
; In units where GM = 1, the orbital period T satisfies T² = a³,
; where a is the semi-major axis.  We evaluate this for four bodies
; with semi-major axes a = [0.5, 1.0, 2.0, 4.0].
;
; The computation is entirely vectorized:
;   V1 = V0 * V0        (a²)
;   V2 = V1 * V0        (a³ = T²)
;
; All values are exact powers of two so there is no rounding.
; T² results: [0.125, 1.0, 8.0, 64.0]
;
; Expect: S1=0x3FFE800000000000 S2=0x4001800000000000 S3=0x4004800000000000 S4=0x4007800000000000

    ; S7 = 2^47 = shared coefficient for all values below (.5 fraction, normalized)
    si 7, 1
    sshl 7, 47

    ; -----------------------------------------------------------------------
    ; Store a[0..3] to words 50-53 (A0 = 0 throughout, absolute offsets)
    ; All share coeff = S7; only the biased exponent (bits 62:48) differs.
    ; a[i] = 0.5 * 2^i  =>  exp = 16384 + i
    ; -----------------------------------------------------------------------

    ; a[0] = 0.5  = 0x4000_8000_0000_0000  (exp = 16384 = 0x4000)
    si 1, 16384
    sshl 1, 48
    sadd 1, 1, 7
    stores 1, 0, 50

    ; a[1] = 1.0  = 0x4001_8000_0000_0000  (exp = 16385 = 0x4001)
    si 1, 16385
    sshl 1, 48
    sadd 1, 1, 7
    stores 1, 0, 51

    ; a[2] = 2.0  = 0x4002_8000_0000_0000  (exp = 16386 = 0x4002)
    si 1, 16386
    sshl 1, 48
    sadd 1, 1, 7
    stores 1, 0, 52

    ; a[3] = 4.0  = 0x4003_8000_0000_0000  (exp = 16387 = 0x4003)
    si 1, 16387
    sshl 1, 48
    sadd 1, 1, 7
    stores 1, 0, 53

    ; -----------------------------------------------------------------------
    ; Vector setup and load
    ; -----------------------------------------------------------------------
    ai 1, 4
    setvl 1          ; VL = 4

    ai 0, 50
    vload 0, 0       ; V0[0..3] = a[0..3]

    ; -----------------------------------------------------------------------
    ; Kepler core
    ; -----------------------------------------------------------------------
    vfmulv 1, 0, 0   ; V1[i] = a[i]²
    vfmulv 2, 1, 0   ; V2[i] = a[i]³  =  T²[i]

    ; -----------------------------------------------------------------------
    ; Extract T² into S1-S4 for verification
    ; -----------------------------------------------------------------------
    ai 0, 0
    vget 1, 2, 0     ; S1 = T²[0] = 0.125
    ai 0, 1
    vget 2, 2, 0     ; S2 = T²[1] = 1.0
    ai 0, 2
    vget 3, 2, 0     ; S3 = T²[2] = 8.0
    ai 0, 3
    vget 4, 2, 0     ; S4 = T²[3] = 64.0

    exit
