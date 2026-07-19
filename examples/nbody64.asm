; nbody64.asm: Direct O(N²) gravitational force computation for 64 bodies.
;
; Computes the net acceleration on every body from every other body using
; vector operations. N=64 matches the Cray-1's maximum vector length, so each
; vector instruction sweeps all 64 source bodies at once. The outer loop runs
; once per target body (64 iterations), giving 4096 evaluated pairs.
;
; Positions (posx/posy/posz) in AU, masses (gmass = G*m) in solar units.
;
; The inverse distance is approximated using a fast reciprocal square root
; seed followed by two Newton-Raphson iterations. Each 64-element dot
; product is reduced with six rotate-and-add passes.
;
; Expect: VL=64 S1=0x3fed800000000000 S4=0x4000800000000000 S7=0xc001800000000000 S2=0xbffde9f5fc182b9c cycles=7895
;         VL    eps^2 = 2^-20         0.5                   -1.0                  accz[0] ≈ -0.114 (body 0 net z-accel; reference -0.120)

    ; VL = 64, full vector width for all source-body sweeps
    ai_l  a1, 64
    setvl a1

    ; Load X, Y, Z position vectors into resident registers v0-v2
    ai_l  a0, posx / 8
    vload v0, a0                  ; v0 = X[0..63]
    ai_l  a0, posy / 8
    vload v1, a0                  ; v1 = Y[0..63]
    ai_l  a0, posz / 8
    vload v2, a0                  ; v2 = Z[0..63]

    ; Load scalar constants
    ai_l  a5, const_eps2 / 8
    loads s1, a5, 0               ; s1 = eps^2
    ai_l  a5, const_half / 8
    loads s4, a5, 0               ; s4 = 0.5
    ai_l  a5, const_3half / 8
    loads s5, a5, 0               ; s5 = 1.5
    ai_l  a5, const_magic / 8
    loads s6, a5, 0               ; s6 = rsqrt seed
    ai_l  a5, const_negone / 8
    loads s7, a5, 0               ; s7 = -1.0

    ai_l  a5, gmass / 8           ; a5 = gmass base (kept throughout)
    ai    a2, 1
    ai    a3, 0
    ai_l  a0, 64                  ; a0 = outer loop counter, 64..1

body_loop:
    asub  a1, a0, a2              ; a1 = body index i (0..63)

    ; Separations: body i vs. all 64 source bodies
    vget  s2, v0, a1
    vfsub v4, s2, v0              ; v4 = xi - X[]
    vget  s2, v1, a1
    vfsub v5, s2, v1              ; v5 = yi - Y[]
    vget  s2, v2, a1
    vfsub v6, s2, v2              ; v6 = zi - Z[]

    ; r^2 + eps^2  (self-term becomes eps^2, no branch needed)
    vfmulv v3, v4, v4
    vfmulv v7, v5, v5
    vfaddv v3, v3, v7
    vfmulv v7, v6, v6
    vfaddv v3, v3, v7
    vfadd  v3, s1, v3             ; v3 = r^2 + eps^2

    ; rsqrt via bit-hack seed + three Newton-Raphson passes: y <- y*(1.5 - 0.5*r2*y*y)
    ai    a4, 1
    vshr  v7, v3, a4              ; bits >> 1
    vsub  v7, s6, v7              ; y0 = magic - (bits >> 1)
    vfmulv v4, v7, v7
    vfmulv v4, v3, v4
    vfmul  v4, s4, v4
    vfsub  v4, s5, v4
    vfmulv v7, v7, v4             ; y1
    vfmulv v4, v7, v7
    vfmulv v4, v3, v4
    vfmul  v4, s4, v4
    vfsub  v4, s5, v4
    vfmulv v7, v7, v4             ; y2
    vfmulv v4, v7, v7
    vfmulv v4, v3, v4
    vfmul  v4, s4, v4
    vfsub  v4, s5, v4
    vfmulv v7, v7, v4             ; v7 = 1/sqrt(r^2 + eps^2)

    ; weight[j] = G*m[j] * invr^3[j]
    vfmulv v5, v7, v7             ; invr^2
    vfmulv v6, v5, v7             ; invr^3
    aadd  a7, a0, a3              ; save loop counter
    aadd  a0, a5, a3              ; a0 = gmass base
    vload v3, a0                  ; v3 = G*m[]
    aadd  a0, a7, a3              ; restore loop counter
    vfmulv v5, v6, v3             ; v5 = weight[]

    ; accx[i] = -sum(dx * weight), reduced via 6 rotate-and-add steps
    vget  s2, v0, a1
    vfsub v4, s2, v0              ; dx[]
    vfmulv v4, v4, v5
    ai a4, 32
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    ai a4, 16
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    ai a4, 8
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    ai a4, 4
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    ai a4, 2
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    ai a4, 1
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    vget  s2, v4, a3
    fmul  s2, s7, s2              ; negate (accel = -sum)
    ai_l  a4, accx / 8
    aadd  a4, a4, a1
    stores s2, a4, 0

    ; accy[i] = -sum(dy * weight)
    vget  s2, v1, a1
    vfsub v4, s2, v1              ; dy[]
    vfmulv v4, v4, v5
    ai a4, 32
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    ai a4, 16
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    ai a4, 8
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    ai a4, 4
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    ai a4, 2
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    ai a4, 1
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    vget  s2, v4, a3
    fmul  s2, s7, s2
    ai_l  a4, accy / 8
    aadd  a4, a4, a1
    stores s2, a4, 0

    ; accz[i] = -sum(dz * weight)
    vget  s2, v2, a1
    vfsub v4, s2, v2              ; dz[]
    vfmulv v4, v4, v5
    ai a4, 32
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    ai a4, 16
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    ai a4, 8
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    ai a4, 4
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    ai a4, 2
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    ai a4, 1
    vrotr v7, v4, a4
    vfaddv v4, v4, v7
    vget  s2, v4, a3
    fmul  s2, s7, s2
    ai_l  a4, accz / 8
    aadd  a4, a4, a1
    stores s2, a4, 0

    asub a0, a0, a2
    jan  body_loop                ; continue while a0 != 0

    exit

; Physical constants encoded as Cray-1 64-bit FP (cray_exp = f64_exp + 15362).
; fast-inverse-sqrt seed is the Cray-1 FP equivalent of the Quake III magic.
#align 64
const_eps2:   #d64 0x3FED800000000000   ; eps^2 = 2^-20 (softening)
const_half:   #d64 0x4000800000000000   ; 0.5
const_3half:  #d64 0x4001C00000000000   ; 1.5
const_magic:  #d64 0x6002300000000000   ; fast-inverse-sqrt seed (Cray-1 FP)
const_negone: #d64 0xC001800000000000   ; -1.0

; 64-body system: positions in AU, G*m in solar units (seed 0xDEADBEEFCAFE1234).
posx:
    #d64 0xC003B78D2491B098   ; -5.735979
    #d64 0x400497086BBEAD26   ;  9.439556
    #d64 0xC001A9D8F79EA063   ; -1.326934
    #d64 0x40048DF537686255   ;  8.872367
    #d64 0x4002EBEAAB2E3A12   ;  3.686198
    #d64 0x4003860B5B56A030   ;  4.188886
    #d64 0xC00089150D7FB772   ; -0.535477
    #d64 0xC0049C43487812FA   ; -9.766427
    #d64 0x400485F1BD172D91   ;  8.371518
    #d64 0xC003DE6D899A2518   ; -6.950871
    #d64 0xC002B6876C8EF5E5   ; -2.852016
    #d64 0xC003BAFE5226316E   ; -5.843545
    #d64 0x4003D6F33C49E5B2   ;  6.717192
    #d64 0x40039FF67EF55825   ;  4.998840
    #d64 0xC000C44F21CA5B76   ; -0.766832
    #d64 0x4001BE16A35228B0   ;  1.485066
    #d64 0x4003F6130041BE8B   ;  7.689819
    #d64 0x4003EEC6AEB624E2   ;  7.461753
    #d64 0xC003FF309036F74A   ; -7.974678
    #d64 0xC0009F42D5C016D9   ; -0.622114
    #d64 0xC002D90709A498DA   ; -3.391055
    #d64 0x400390AEB90F0B93   ;  4.521328
    #d64 0xC002867D68021087   ; -2.101404
    #d64 0x4001E3743EE5168F   ;  1.776985
    #d64 0x4003E903E570749B   ;  7.281726
    #d64 0x4003FE381D489174   ;  7.944350
    #d64 0xC0048D4768A2E6C1   ; -8.829934
    #d64 0xC0038E022D0D56AE   ; -4.437766
    #d64 0x4003E9919F3FD2DC   ;  7.299026
    #d64 0xC00084E96002D7EC   ; -0.519186
    #d64 0x4003C2A6D06BDC6D   ;  6.082863
    #d64 0x400492594C7AB28C   ;  9.146801
    #d64 0xC003CF9EEADE7BF3   ; -6.488149
    #d64 0x4001DCA28FFF3E1B   ;  1.723711
    #d64 0xC003A0A5816F4FB3   ; -5.020203
    #d64 0x4002D137773CEA0D   ;  3.269010
    #d64 0x4003DCA43A95631B   ;  6.895047
    #d64 0x400392150C8EDBCD   ;  4.565069
    #d64 0xC002FF189F6CC92B   ; -3.985878
    #d64 0x4002D3CEE52B2CA4   ;  3.309503
    #d64 0xC00283DC1B7C6EE3   ; -2.060309
    #d64 0xC00286508D21F817   ; -2.098666
    #d64 0x400193760ACBE654   ;  1.152040
    #d64 0xC00394B3B3721A89   ; -4.646936
    #d64 0xC0048D31B8B9E5C4   ; -8.824639
    #d64 0xC003BD97FF7627A0   ; -5.924804
    #d64 0xC00480A91252622F   ; -8.041277
    #d64 0xC002DA75FDDCCB08   ; -3.413452
    #d64 0xC000CB20863EDF96   ; -0.793465
    #d64 0x40038E0BED7531CA   ;  4.438956
    #d64 0xC00480BF3066CEB6   ; -8.046677
    #d64 0x4002D0B81C4D0AA9   ;  3.261237
    #d64 0xBFFCA7D8F8398930   ; -0.040978
    #d64 0xC002F4D67E16BF10   ; -3.825592
    #d64 0x4003CA82E28E596A   ;  6.328477
    #d64 0xC000F1CB9E879CF0   ; -0.944513
    #d64 0xC0029D1A5547D023   ; -2.454732
    #d64 0x4001D1A60573AA1F   ;  1.637879
    #d64 0x4003FCB8840CD78C   ;  7.897524
    #d64 0x4002BB156D471E2B   ;  2.923183
    #d64 0xC000C23EFA515325   ; -0.758773
    #d64 0x40048FB9B6BA25B2   ;  8.982840
    #d64 0x4003EEF862D215C9   ;  7.467821
    #d64 0xC0029F06308BA299   ; -2.484753
posy:
    #d64 0x4003C9BC2703C650   ;  6.304218
    #d64 0xBFFF8CAAECEB9A24   ; -0.274742
    #d64 0x4003C149D584400F   ;  6.040263
    #d64 0x3FFEE1DBFDE3E648   ;  0.220566
    #d64 0x4002D14532385FE7   ;  3.269848
    #d64 0x40048C6267D97F25   ;  8.774025
    #d64 0x400485A3E08716D5   ;  8.352509
    #d64 0x40049BF2CD5EE120   ;  9.746778
    #d64 0x400382706ADE5B4D   ;  4.076223
    #d64 0xC001CD43439B6E43   ; -1.603615
    #d64 0xC001C8246ADDD720   ; -1.563611
    #d64 0x400493247DED7731   ;  9.196409
    #d64 0x4001FE82BA13CC15   ;  1.988364
    #d64 0xC002EFF17AC29F45   ; -3.749114
    #d64 0x4002F01C598E724C   ;  3.751730
    #d64 0xC001A8D577F46F12   ; -1.319015
    #d64 0xC003BAAAD91021C7   ; -5.833355
    #d64 0x4003FE75B8A9B311   ;  7.951870
    #d64 0x3FFFD9770D15DE89   ;  0.424736
    #d64 0xC0049A0A30F7360D   ; -9.627488
    #d64 0xC0038AC004A1EF47   ; -4.335940
    #d64 0xC0038A5FB57C48DF   ; -4.324183
    #d64 0xC003EA97D662ED44   ; -7.331035
    #d64 0x40019121A307D1E5   ;  1.133839
    #d64 0xC000BC2D16E4A905   ; -0.735063
    #d64 0xC0048A21F35F5FFA   ; -8.633289
    #d64 0xC001B467C86E1FC2   ; -1.409417
    #d64 0x40018D2D4CE53ECE   ;  1.102945
    #d64 0x40029B710FE8C61E   ;  2.428776
    #d64 0xC003D85CDFDBE551   ; -6.761337
    #d64 0xC001CD41863E65AC   ; -1.603562
    #d64 0x4001DD75D2360EC1   ;  1.730158
    #d64 0xBFFF98304F72D379   ; -0.297244
    #d64 0xC001806D9192BCA8   ; -1.003344
    #d64 0x40049AC4AB72D6F2   ;  9.673015
    #d64 0xC003883FFF1EA32F   ; -4.257812
    #d64 0xC001DE4AF71C7A68   ; -1.736663
    #d64 0x4002C94F241963DF   ;  3.145455
    #d64 0xC00499E47231CB1B   ; -9.618273
    #d64 0x4003BCEB07A049B9   ;  5.903690
    #d64 0x3FFE9C6C3E54CEB0   ;  0.152757
    #d64 0x4003ECC5DFEF02DC   ;  7.399155
    #d64 0x4003B598852D9829   ;  5.674868
    #d64 0x4003E50DB1F2CB7B   ;  7.157922
    #d64 0x4003FB7DF11CBDB3   ;  7.859124
    #d64 0x400395B8848906E1   ;  4.678774
    #d64 0xC004982C67D72335   ; -9.510841
    #d64 0xC000B8FEC946E188   ; -0.722638
    #d64 0xC004868FC5DB2CAF   ; -8.410101
    #d64 0xC002B3FEF9EE9888   ; -2.812438
    #d64 0x40039AB17843E669   ;  4.834164
    #d64 0x400491E19FBEF1BC   ;  9.117584
    #d64 0xBFFFBE20C5696392   ; -0.371344
    #d64 0x4003852A81B702D1   ;  4.161439
    #d64 0xC002D1F47C813ABC   ; -3.280547
    #d64 0xC00295ED027FCA65   ; -2.342591
    #d64 0xC003DBA50F75627D   ; -6.863899
    #d64 0x4001A7B1F7BFC60C   ;  1.310119
    #d64 0x4003F24DAD7A5A3B   ;  7.571982
    #d64 0x4001BC2BD774393C   ;  1.470088
    #d64 0x4004832578CCC245   ;  8.196648
    #d64 0x4002CE0B70B6C53E   ;  3.219448
    #d64 0x4003CA2DE5571043   ;  6.318103
    #d64 0xC003979EEAB88E8C   ; -4.738149
posz:
    #d64 0x4000FA46901D003D   ;  0.977639
    #d64 0x40048AB682655979   ;  8.669558
    #d64 0xC003E63A1BFA9581   ; -7.194593
    #d64 0xC003A53B55B9B67B   ; -5.163493
    #d64 0x4002E1D099AF5E8B   ;  3.528357
    #d64 0x40038187F937F2FD   ;  4.047848
    #d64 0x40049C55A10CB1DA   ;  9.770906
    #d64 0xC0049C437A2A22AF   ; -9.766474
    #d64 0xC003B8688CFA48E6   ; -5.762763
    #d64 0x40048D0DD7676BDF   ;  8.815879
    #d64 0xC003F66469B07162   ; -7.699757
    #d64 0x40048A5916821071   ;  8.646750
    #d64 0x400283DC41D095A7   ;  2.060318
    #d64 0xC000A74641DB5A7E   ; -0.653416
    #d64 0x40018DA3CDDDCD7F   ;  1.106561
    #d64 0x4003A80D6E81F79A   ;  5.251640
    #d64 0xC0049EB01E100791   ; -9.917997
    #d64 0x4004993E5C197866   ;  9.577725
    #d64 0xC00380C96E1BCE35   ; -4.024589
    #d64 0x4001DD7C82BCC2D9   ;  1.730362
    #d64 0xC003D6AC911CE312   ; -6.708565
    #d64 0xC003F527B3B3F732   ; -7.661096
    #d64 0x40048811E97B3909   ;  8.504373
    #d64 0xC003D935504D41BD   ; -6.787758
    #d64 0xC003BD7F6A672613   ; -5.921804
    #d64 0x400485115D17A839   ;  8.316739
    #d64 0xC003B7D9B410B1BB   ; -5.745325
    #d64 0x400388247EE1300C   ;  4.254455
    #d64 0xC002E1FD8B7C4535   ; -3.531100
    #d64 0x4003FA1CB28D84FD   ;  7.816003
    #d64 0xC003A96A939D81B7   ; -5.294260
    #d64 0xC00393A69786989F   ; -4.614086
    #d64 0xC001D44724128577   ; -1.658421
    #d64 0x4003E8B4C44062BA   ;  7.272066
    #d64 0x400480E64C775849   ;  8.056225
    #d64 0x4002EB5DEDF505E7   ;  3.677608
    #d64 0x40049F8F2A3455B3   ;  9.972452
    #d64 0xC00382D6BEDBF046   ; -4.088714
    #d64 0x40048B15197ECCB3   ;  8.692651
    #d64 0x40048F6273C7226E   ;  8.961536
    #d64 0xC001BD4C274FAEC7   ; -1.478887
    #d64 0x40039EA2D4FB4D9D   ;  4.957377
    #d64 0xC003A463552E1D38   ; -5.137126
    #d64 0xC003C631A44CAB41   ; -6.193560
    #d64 0x40049CAFF136B55D   ;  9.792955
    #d64 0xC00494261275ADB9   ; -9.259295
    #d64 0xC004958955A41C81   ; -9.346029
    #d64 0x4002CBD2B389C004   ;  3.184735
    #d64 0x4003B6EA009BEA7C   ;  5.716065
    #d64 0xC003CE01381A26B6   ; -6.437649
    #d64 0xC00486F3EB4D2020   ; -8.434551
    #d64 0xC003905BFFAEC5BF   ; -4.511230
    #d64 0xC0049B781ADE7B80   ; -9.716822
    #d64 0xC003BAD499FBDFA6   ; -5.838452
    #d64 0x4001B8650C755007   ;  1.440584
    #d64 0x400289C1130E7F7A   ;  2.152409
    #d64 0xC0018337A4CAEF3C   ; -1.025136
    #d64 0xC002B5012E3ABF41   ; -2.828197
    #d64 0x4001E21D7622B977   ;  1.766524
    #d64 0xC002D0A2930B47E2   ; -3.259923
    #d64 0x400483E35506D577   ;  8.243001
    #d64 0x40048D642E5AFAC5   ;  8.836958
    #d64 0x40048C181C24230C   ;  8.755886
    #d64 0xC003CDEE3C48C924   ; -6.435331
gmass:
    #d64 0x4001EAABBC5780FD   ;  1.833366
    #d64 0x4001A5F9872FEBDB   ;  1.296677
    #d64 0x400387FC46C91B6C   ;  4.249545
    #d64 0x40038331E26D8DD3   ;  4.099839
    #d64 0x4000A7674BCBE1E7   ;  0.653920
    #d64 0x4002995F5E6CAEAD   ;  2.396446
    #d64 0x4001E886F7247833   ;  1.816619
    #d64 0x4002D78F19F6BC92   ;  3.368109
    #d64 0x4000C74086466C6B   ;  0.778328
    #d64 0x4002CFD171267265   ;  3.247158
    #d64 0x4001EC6FB58D6A81   ;  1.847159
    #d64 0x40029CA8F54C24E2   ;  2.447812
    #d64 0x4002BAEF2EBE7EAC   ;  2.920849
    #d64 0x4002DBA31187ED45   ;  3.431828
    #d64 0x4002BD2B7A483720   ;  2.955779
    #d64 0x4002D479035299F4   ;  3.319886
    #d64 0x4002D6585A8C2CF3   ;  3.349143
    #d64 0x4001D3C8CB0024F7   ;  1.654565
    #d64 0x4002845C283E6EA6   ;  2.068125
    #d64 0x40028D79EB5649F4   ;  2.210566
    #d64 0x40029F6328D998C8   ;  2.490427
    #d64 0x4001C8272372C778   ;  1.563694
    #d64 0x400387310C31E00A   ;  4.224737
    #d64 0x40028462E52D6CE1   ;  2.068536
    #d64 0x4001DADE7D82F043   ;  1.709915
    #d64 0x4002FF059E16DFA6   ;  3.984718
    #d64 0x4001FB92B48F05CC   ;  1.965415
    #d64 0x4000F80FAE6C2537   ;  0.968989
    #d64 0x3FFFB66E11BD0244   ;  0.356309
    #d64 0x4002FFE05D6DF60C   ;  3.998069
    #d64 0x4001CBD771ADAB7F   ;  1.592512
    #d64 0x400298FA34D80973   ;  2.390271
    #d64 0x4001CD5921093A38   ;  1.604283
    #d64 0x4002B2CE65CB68ED   ;  2.793848
    #d64 0x400296F2FD1D20D8   ;  2.358581
    #d64 0x4003848E16735751   ;  4.142345
    #d64 0x400397017C7A588A   ;  4.718931
    #d64 0x4002CD6705CE6F7D   ;  3.209413
    #d64 0x40029B36BA8B5F02   ;  2.425215
    #d64 0x4001C1CC494BD8CE   ;  1.514047
    #d64 0x40038B9BEE737044   ;  4.362785
    #d64 0x3FFF845428815D7F   ;  0.258455
    #d64 0x3FFFC8702DB49E94   ;  0.391481
    #d64 0x4001AFDCF019F934   ;  1.373930
    #d64 0x4002CAE3A6CDD2B5   ;  3.170145
    #d64 0x4002C8373C93F134   ;  3.128371
    #d64 0x40039F544B4B9EAB   ;  4.979040
    #d64 0x40009B57A0E7E403   ;  0.606806
    #d64 0x40039C68B54130B7   ;  4.887782
    #d64 0x4003998C1999E597   ;  4.798352
    #d64 0x3FFED565766AB2D3   ;  0.208395
    #d64 0x4002D8451F0BA872   ;  3.379219
    #d64 0x400390CE63A01F61   ;  4.525194
    #d64 0x4002D5DA33B8E929   ;  3.341443
    #d64 0x4002BF5CFA458C87   ;  2.990050
    #d64 0x4002E1C580B88F4B   ;  3.527680
    #d64 0x40029596AC702640   ;  2.337321
    #d64 0x400296E7AEF2BD58   ;  2.357891
    #d64 0x4001A2871C5E4457   ;  1.269748
    #d64 0x4002DD3E5C6A1ADF   ;  3.456931
    #d64 0x4002AC0CEFC237AB   ;  2.688290
    #d64 0x3FFFA5BDC8C025CA   ;  0.323714
    #d64 0x4001E51E078C9F2F   ;  1.789979
    #d64 0x400394B85F9A03C8   ;  4.647507
accx:  #res 64*64
accy:  #res 64*64
accz:  #res 64*64
