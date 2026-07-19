; kepler.asm: Kepler's Third Law: T² = a³
;
; In units where GM = 1, the orbital period T of a body satisfies T² = a³,
; where a is the semi-major axis.  We evaluate this for all eight planets
; using two vector instructions.
;
; Semi-major axes (AU) are stored as Cray-1 FP constants after the program.
; The computation is entirely vectorized:
;   V1 = V0 * V0   (a²)
;   V2 = V1 * V0   (a³ = T²)
;
; Expect: S0=0x3ffced96c0558990 S1=0x3fffc1c4b722bb5e S2=0x4001800000000000 S3=0x4002e26443b0dbd5 S4=0x40088cd1bb180d2e S5=0x400ad8dcdbd5f522 S6=0x400ddccf4c045104 S7=0x400fd46a770698b7
;         Mercury               Venus                 Earth                 Mars                  Jupiter               Saturn                Uranus                Neptune
; T²      0.058 yr²             0.378 yr²             1.000 yr²             3.537 yr²             140.8 yr²             867.5 yr²             7066 yr²              27189 yr²

    ; VL = 8 (one element per planet)
    ai 1, 8
    setvl 1

    ; Load semi-major axes from the data table into V0
    ai_l 0, semi_major_axes / 8
    vload 0, 0

    ; Kepler core: two vector multiplies
    vfmulv 1, 0, 0   ; V1[i] = a[i]²
    vfmulv 2, 1, 0   ; V2[i] = a[i]³  =  T²[i]

    ; Extract T² results into S0-S7
    ai 0, 0
    vget 0, 2, 0     ; S0 = T²[Mercury]
    ai 0, 1
    vget 1, 2, 0     ; S1 = T²[Venus]
    ai 0, 2
    vget 2, 2, 0     ; S2 = T²[Earth]
    ai 0, 3
    vget 3, 2, 0     ; S3 = T²[Mars]
    ai 0, 4
    vget 4, 2, 0     ; S4 = T²[Jupiter]
    ai 0, 5
    vget 5, 2, 0     ; S5 = T²[Saturn]
    ai 0, 6
    vget 6, 2, 0     ; S6 = T²[Uranus]
    ai 0, 7
    vget 7, 2, 0     ; S7 = T²[Neptune]

    exit

; Semi-major axes of the eight planets in AU, encoded as Cray-1 64-bit FP.
; Values from the IAU mean orbital elements (J2000.0 epoch).
#align 8
semi_major_axes:
    #d64 0x3fffc631d712a0ec   ; Mercury   0.387 AU
    #d64 0x4000b92c49342678   ; Venus     0.723 AU
    #d64 0x4001800000000000   ; Earth     1.000 AU
    #d64 0x4001c307e9d94d0d   ; Mars      1.524 AU
    #d64 0x4003a67bb9496249   ; Jupiter   5.203 AU
    #d64 0x40049897d6b65a9a   ; Saturn    9.537 AU
    #d64 0x4005998368f08461   ; Uranus   19.19 AU
    #d64 0x4005f08f33ca31e7   ; Neptune  30.07 AU
