; Cray-1 instruction set ruleset for customasm.
;
; Each instruction is one 16-bit parcel (single) or two parcels (long, marked L).
; Parcels are big-endian. Four parcels fit in one 64-bit memory word.
; The 7-bit opcode is g(4):h(3) occupying bits 15:9 of the first parcel.
;
; With #bits 8, labels are byte offsets. Branch targets divide by 2 to convert
; to parcel indices, since each parcel is 2 bytes.

#bankdef default
{
    #bits 8
    #addr 0
    #size 0x800000  ; 8M bytes = 1M 64-bit words
    #outp 0
}

#subruledef reg_a
{
    a0 => 0
    a1 => 1
    a2 => 2
    a3 => 3
    a4 => 4
    a5 => 5
    a6 => 6
    a7 => 7
    {n: u3} => n
}

#subruledef reg_s
{
    s0 => 0
    s1 => 1
    s2 => 2
    s3 => 3
    s4 => 4
    s5 => 5
    s6 => 6
    s7 => 7
    {n: u3} => n
}

#subruledef reg_v
{
    v0 => 0
    v1 => 1
    v2 => 2
    v3 => 3
    v4 => 4
    v5 => 5
    v6 => 6
    v7 => 7
    {n: u3} => n
}

#ruledef
{
    ; -----------------------------------------------------------------------
    ; Control
    ; -----------------------------------------------------------------------

    ; 004: normal exit
    exit => 0b0000100`7 @ 0`9

    ; -----------------------------------------------------------------------
    ; Vector length and mask
    ; -----------------------------------------------------------------------

    ; 020 VL = Ak
    setvl {k: reg_a}          => 0b0010000`7 @ 0`6 @ k`3

    ; 033 VM = Sj
    setvm {j: reg_s}           => 0b0011011`7 @ 0`3 @ j`3 @ 0`3

    ; 034 VM = 0
    clrvm                      => 0b0011100`7 @ 0`9

    ; 073 Si = VM
    vmread {i: reg_s}          => 0b0111011`7 @ i`3 @ 0`6

    ; -----------------------------------------------------------------------
    ; Vector element access
    ; -----------------------------------------------------------------------

    ; 076 Si = Vj[Ak]
    vget {i: reg_s}, {j: reg_v}, {k: reg_a}  => 0b0111110`7 @ i`3 @ j`3 @ k`3

    ; 077 Vi[Ak] = Sj
    vput {i: reg_v}, {j: reg_s}, {k: reg_a}  => 0b0111111`7 @ i`3 @ j`3 @ k`3

    ; -----------------------------------------------------------------------
    ; Address register (A) — 24-bit
    ; -----------------------------------------------------------------------

    ; 022 Ai = jk: load 6-bit constant into Ai (single parcel)
    ai {i: reg_a}, {v: u6}               => 0b0010010`7 @ i`3 @ v`6

    ; 021 Ai = v: load 22-bit constant into Ai (long)
    ai_l {i: reg_a}, {v: u22}            => 0b0010001`7 @ i`3 @ (v >> 16)`6  @ (v & 0xffff)`16

    ; 030 Ai = Aj + Ak
    aadd {i: reg_a}, {j: reg_a}, {k: reg_a}   => 0b0011000`7 @ i`3 @ j`3 @ k`3

    ; 031 Ai = Aj - Ak
    asub {i: reg_a}, {j: reg_a}, {k: reg_a}   => 0b0011001`7 @ i`3 @ j`3 @ k`3

    ; 032 Ai = Aj * Ak (lower 24 bits)
    amul {i: reg_a}, {j: reg_a}, {k: reg_a}   => 0b0011010`7 @ i`3 @ j`3 @ k`3

    ; 023 Ai = Sj (lower 24 bits)
    a_s {i: reg_a}, {j: reg_s}                => 0b0010011`7 @ i`3 @ j`3 @ 0`3

    ; -----------------------------------------------------------------------
    ; B register (24-bit intermediate address registers, B00-B77)
    ; The 6-bit index n encodes as j=n>>3, k=n&7 in the parcel.
    ; -----------------------------------------------------------------------

    ; 024 Ai = Bjk
    ldb {i: reg_a}, {n: u6}  => 0b0010100`7 @ i`3 @ (n >> 3)`3 @ (n & 0b111)`3

    ; 025 Bjk = Ai
    stb {i: reg_a}, {n: u6}  => 0b0010101`7 @ i`3 @ (n >> 3)`3 @ (n & 0b111)`3

    ; 005 with i=0: J Bjk — jump to parcel address held in B register n
    jb {n: u6}               => 0b0000101`7 @ 0b000`3 @ (n >> 3)`3 @ (n & 0b111)`3

    ; -----------------------------------------------------------------------
    ; Scalar register (S) — 64-bit
    ; -----------------------------------------------------------------------

    ; 040 Si = v: load 22-bit constant into Si (long)
    si {i: reg_s}, {v: u22}              => 0b0100000`7 @ i`3 @ (v >> 16)`6  @ (v & 0xffff)`16

    ; 043 Si = 0
    sclr {i: reg_s}                      => 0b0100011`7 @ i`3 @ 0`6

    ; 071 Si = Ak (zero-extend from 24-bit)
    s_a {i: reg_s}, {k: reg_a}           => 0b0111001`7 @ i`3 @ 0b000`3 @ k`3

    ; 054 Si <<= jk  (shift Si left by jk places in-place)
    sshl {i: reg_s}, {jk: u6}            => 0b0101100`7 @ i`3 @ jk`6

    ; 055 Si = Si >> (64-jk)  (shift Si right, complement of sshl)
    sshr {i: reg_s}, {jk: u6}            => 0b0101101`7 @ i`3 @ jk`6

    ; 060 Si = Sj + Sk
    sadd {i: reg_s}, {j: reg_s}, {k: reg_s}   => 0b0110000`7 @ i`3 @ j`3 @ k`3

    ; 061 Si = Sj - Sk
    ssub {i: reg_s}, {j: reg_s}, {k: reg_s}   => 0b0110001`7 @ i`3 @ j`3 @ k`3

    ; -----------------------------------------------------------------------
    ; Scalar logical
    ; -----------------------------------------------------------------------

    ; 044 Si = Sj & Sk
    sand  {i: reg_s}, {j: reg_s}, {k: reg_s}  => 0b0100100`7 @ i`3 @ j`3 @ k`3
    ; 045 Si = Sj & ~Sk
    sandc {i: reg_s}, {j: reg_s}, {k: reg_s}  => 0b0100101`7 @ i`3 @ j`3 @ k`3
    ; 046 Si = Sj ^ Sk
    sxor  {i: reg_s}, {j: reg_s}, {k: reg_s}  => 0b0100110`7 @ i`3 @ j`3 @ k`3
    ; 047 Si = ~(Sj ^ Sk)  (equivalence / XNOR)
    sxeqv {i: reg_s}, {j: reg_s}, {k: reg_s}  => 0b0100111`7 @ i`3 @ j`3 @ k`3
    ; 050 Si = (Si & ~Sk) | (Sj & Sk)
    smerge  {i: reg_s}, {j: reg_s}, {k: reg_s} => 0b0101000`7 @ i`3 @ j`3 @ k`3
    ; 051 Si = (Si & ~mask) | (Sj & mask)  where mask = sign bit of Sj broadcast to all bits
    smerges {i: reg_s}, {j: reg_s}             => 0b0101001`7 @ i`3 @ j`3 @ 0`3

    ; -----------------------------------------------------------------------
    ; Scalar floating point
    ; -----------------------------------------------------------------------

    ; 062 Si = Sj + Sk (FP add; with j=s0 and S0=0: normalizes Sk)
    fadd {i: reg_s}, {j: reg_s}, {k: reg_s}   => 0b0110010`7 @ i`3 @ j`3 @ k`3

    ; 063 Si = Sj - Sk (FP sub; with j=s0 and S0=0: negates and normalizes Sk)
    fsub {i: reg_s}, {j: reg_s}, {k: reg_s}   => 0b0110011`7 @ i`3 @ j`3 @ k`3

    ; 064 Si = Sj * Sk (FP multiply, truncated)
    fmul {i: reg_s}, {j: reg_s}, {k: reg_s}   => 0b0110100`7 @ i`3 @ j`3 @ k`3

    ; 065 Si = Sj * Sk (half-precision rounded)
    fmulh {i: reg_s}, {j: reg_s}, {k: reg_s}  => 0b0110101`7 @ i`3 @ j`3 @ k`3

    ; 066 Si = Sj * Sk (full-precision rounded)
    fmulr {i: reg_s}, {j: reg_s}, {k: reg_s}  => 0b0110110`7 @ i`3 @ j`3 @ k`3

    ; 067 Si = 2 * Sj * Sk
    fmul2 {i: reg_s}, {j: reg_s}, {k: reg_s}  => 0b0110111`7 @ i`3 @ j`3 @ k`3

    ; 070 Si = reciprocal approximation of Sj (k field unused)
    frecip {i: reg_s}, {j: reg_s}              => 0b0111000`7 @ i`3 @ j`3 @ 0`3

    ; -----------------------------------------------------------------------
    ; Vector floating point multiply
    ; -----------------------------------------------------------------------

    ; 160 Vi = Sj * Vk
    vfmul  {i: reg_v}, {j: reg_s}, {k: reg_v}  => 0b1110000`7 @ i`3 @ j`3 @ k`3
    ; 161 Vi = Vj * Vk
    vfmulv {i: reg_v}, {j: reg_v}, {k: reg_v}  => 0b1110001`7 @ i`3 @ j`3 @ k`3
    ; 162 Vi = Sj *H Vk (half-precision rounded)
    vfmulh  {i: reg_v}, {j: reg_s}, {k: reg_v} => 0b1110010`7 @ i`3 @ j`3 @ k`3
    ; 163 Vi = Vj *H Vk
    vfmulhv {i: reg_v}, {j: reg_v}, {k: reg_v} => 0b1110011`7 @ i`3 @ j`3 @ k`3
    ; 164 Vi = Sj *R Vk (full-precision rounded)
    vfmulr  {i: reg_v}, {j: reg_s}, {k: reg_v} => 0b1110100`7 @ i`3 @ j`3 @ k`3
    ; 165 Vi = Vj *R Vk
    vfmulrv {i: reg_v}, {j: reg_v}, {k: reg_v} => 0b1110101`7 @ i`3 @ j`3 @ k`3
    ; 166 Vi = 2 * Sj * Vk
    vfmul2  {i: reg_v}, {j: reg_s}, {k: reg_v} => 0b1110110`7 @ i`3 @ j`3 @ k`3
    ; 167 Vi = 2 * Vj * Vk
    vfmul2v {i: reg_v}, {j: reg_v}, {k: reg_v} => 0b1110111`7 @ i`3 @ j`3 @ k`3

    ; -----------------------------------------------------------------------
    ; Vector floating point add/sub
    ; -----------------------------------------------------------------------

    ; 170 Vi = Sj + Vk
    vfadd  {i: reg_v}, {j: reg_s}, {k: reg_v}  => 0b1111000`7 @ i`3 @ j`3 @ k`3
    ; 171 Vi = Vj + Vk
    vfaddv {i: reg_v}, {j: reg_v}, {k: reg_v}  => 0b1111001`7 @ i`3 @ j`3 @ k`3
    ; 172 Vi = Sj - Vk
    vfsub  {i: reg_v}, {j: reg_s}, {k: reg_v}  => 0b1111010`7 @ i`3 @ j`3 @ k`3
    ; 173 Vi = Vj - Vk
    vfsubv {i: reg_v}, {j: reg_v}, {k: reg_v}  => 0b1111011`7 @ i`3 @ j`3 @ k`3

    ; 174 Vi = reciprocal approximation of Vj (k unused)
    vfrecip {i: reg_v}, {j: reg_v}              => 0b1111100`7 @ i`3 @ j`3 @ 0`3

    ; -----------------------------------------------------------------------
    ; Branches — all long (32-bit), target is a label (byte address / 2 = parcel index)
    ; -----------------------------------------------------------------------

    ; 006 J exp: unconditional jump
    j {t}   => 0b0000110`7 @ ((t/2) >> 16)`9 @ ((t/2) & 0xffff)`16

    ; 007 R exp: return jump — saves return address in B00, jumps to exp
    ret {t} => 0b0000111`7 @ ((t/2) >> 16)`9 @ ((t/2) & 0xffff)`16

    ; 010 JAZ: branch if A0 = 0
    jaz {t} => 0b0001000`7 @ ((t/2) >> 16)`9 @ ((t/2) & 0xffff)`16

    ; 011 JAN: branch if A0 ≠ 0
    jan {t} => 0b0001001`7 @ ((t/2) >> 16)`9 @ ((t/2) & 0xffff)`16

    ; 012 JAP: branch if A0 positive
    jap {t} => 0b0001010`7 @ ((t/2) >> 16)`9 @ ((t/2) & 0xffff)`16

    ; 013 JAM: branch if A0 negative
    jam {t} => 0b0001011`7 @ ((t/2) >> 16)`9 @ ((t/2) & 0xffff)`16

    ; 014 JSZ: branch if S0 = 0
    jsz {t} => 0b0001100`7 @ ((t/2) >> 16)`9 @ ((t/2) & 0xffff)`16

    ; 015 JSN: branch if S0 ≠ 0
    jsn {t} => 0b0001101`7 @ ((t/2) >> 16)`9 @ ((t/2) & 0xffff)`16

    ; 016 JSP: branch if S0 positive
    jsp {t} => 0b0001110`7 @ ((t/2) >> 16)`9 @ ((t/2) & 0xffff)`16

    ; 017 JSM: branch if S0 negative
    jsm {t} => 0b0001111`7 @ ((t/2) >> 16)`9 @ ((t/2) & 0xffff)`16

    ; -----------------------------------------------------------------------
    ; Memory load/store, all long (32-bit)
    ; Ah is the base address register (encoded in opcode low 3 bits).
    ; addr is a 22-bit word address.
    ; -----------------------------------------------------------------------

    ; 0o100|h  Ai = mem[Ah + addr]
    loada {i: reg_a}, {h: reg_a}, {addr: u22}  => 0b1000`4 @ h`3 @ i`3 @ (addr >> 16)`6 @ (addr & 0xffff)`16

    ; 0o110|h  mem[Ah + addr] = Ai
    storea {i: reg_a}, {h: reg_a}, {addr: u22} => 0b1001`4 @ h`3 @ i`3 @ (addr >> 16)`6 @ (addr & 0xffff)`16

    ; 0o120|h  Si = mem[Ah + addr]
    loads {i: reg_s}, {h: reg_a}, {addr: u22}  => 0b1010`4 @ h`3 @ i`3 @ (addr >> 16)`6 @ (addr & 0xffff)`16

    ; 0o130|h  mem[Ah + addr] = Si
    stores {i: reg_s}, {h: reg_a}, {addr: u22} => 0b1011`4 @ h`3 @ i`3 @ (addr >> 16)`6 @ (addr & 0xffff)`16

    ; -----------------------------------------------------------------------
    ; Vector logical
    ; -----------------------------------------------------------------------

    ; 140 Vi = Sj & Vk
    vand  {i: reg_v}, {j: reg_s}, {k: reg_v}  => 0b1100000`7 @ i`3 @ j`3 @ k`3
    ; 141 Vi = Vj & Vk
    vandv {i: reg_v}, {j: reg_v}, {k: reg_v}  => 0b1100001`7 @ i`3 @ j`3 @ k`3
    ; 142 Vi = Sj | Vk
    vor   {i: reg_v}, {j: reg_s}, {k: reg_v}  => 0b1100010`7 @ i`3 @ j`3 @ k`3
    ; 143 Vi = Vj | Vk
    vorv  {i: reg_v}, {j: reg_v}, {k: reg_v}  => 0b1100011`7 @ i`3 @ j`3 @ k`3
    ; 144 Vi = Sj ^ Vk
    vxor  {i: reg_v}, {j: reg_s}, {k: reg_v}  => 0b1100100`7 @ i`3 @ j`3 @ k`3
    ; 145 Vi = Vj ^ Vk
    vxorv {i: reg_v}, {j: reg_v}, {k: reg_v}  => 0b1100101`7 @ i`3 @ j`3 @ k`3
    ; 146 Vi[n] = VM[n] ? Sj : Vk[n]
    vmerge  {i: reg_v}, {j: reg_s}, {k: reg_v} => 0b1100110`7 @ i`3 @ j`3 @ k`3
    ; 147 Vi[n] = VM[n] ? Vj[n] : Vk[n]
    vmergev {i: reg_v}, {j: reg_v}, {k: reg_v} => 0b1100111`7 @ i`3 @ j`3 @ k`3

    ; -----------------------------------------------------------------------
    ; Vector shift (Ak holds shift count)
    ; -----------------------------------------------------------------------

    ; 150 Vi = Vj << Ak
    vshl  {i: reg_v}, {j: reg_v}, {k: reg_a}  => 0b1101000`7 @ i`3 @ j`3 @ k`3
    ; 151 Vi = Vj >> Ak
    vshr  {i: reg_v}, {j: reg_v}, {k: reg_a}  => 0b1101001`7 @ i`3 @ j`3 @ k`3
    ; 152 Vi = rotl(Vj, Ak)
    vrotl {i: reg_v}, {j: reg_v}, {k: reg_a}  => 0b1101010`7 @ i`3 @ j`3 @ k`3
    ; 153 Vi = rotr(Vj, Ak)
    vrotr {i: reg_v}, {j: reg_v}, {k: reg_a}  => 0b1101011`7 @ i`3 @ j`3 @ k`3

    ; -----------------------------------------------------------------------
    ; Vector integer add
    ; -----------------------------------------------------------------------

    ; 154 Vi = Sj + Vk
    vadd  {i: reg_v}, {j: reg_s}, {k: reg_v}  => 0b1101100`7 @ i`3 @ j`3 @ k`3
    ; 155 Vi = Vj + Vk
    vaddv {i: reg_v}, {j: reg_v}, {k: reg_v}  => 0b1101101`7 @ i`3 @ j`3 @ k`3
    ; 156 Vi = Sj - Vk
    vsub  {i: reg_v}, {j: reg_s}, {k: reg_v}  => 0b1101110`7 @ i`3 @ j`3 @ k`3
    ; 157 Vi = Vj - Vk
    vsubv {i: reg_v}, {j: reg_v}, {k: reg_v}  => 0b1101111`7 @ i`3 @ j`3 @ k`3

    ; -----------------------------------------------------------------------
    ; Vector mask test (result always goes to VM)
    ; -----------------------------------------------------------------------

    ; 175 VM[n] = 1 where Vj[n] == 0
    vmsetz {j: reg_v}  => 0b1111101`7 @ 0`3 @ j`3 @ 0`3
    ; 175 VM[n] = 1 where Vj[n] != 0
    vmsetn {j: reg_v}  => 0b1111101`7 @ 0`3 @ j`3 @ 1`3
    ; 175 VM[n] = 1 where Vj[n] > 0 (positive)
    vmsetp {j: reg_v}  => 0b1111101`7 @ 0`3 @ j`3 @ 2`3
    ; 175 VM[n] = 1 where Vj[n] < 0 (negative)
    vmsetm {j: reg_v}  => 0b1111101`7 @ 0`3 @ j`3 @ 3`3

    ; -----------------------------------------------------------------------
    ; Vector memory load/store (base = A0; k=a0 means stride 1)
    ; -----------------------------------------------------------------------

    ; 176 Vi[n] = mem[A0 + n * Ak]  (k=a0 -> stride 1)
    vload  {i: reg_v}, {k: reg_a}  => 0b1111110`7 @ i`3 @ 0`3 @ k`3
    ; 177 mem[A0 + n * Ak] = Vj[n]  (k=a0 -> stride 1)
    vstore {j: reg_v}, {k: reg_a}  => 0b1111111`7 @ 0`3 @ j`3 @ k`3
}
