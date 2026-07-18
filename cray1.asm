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

#ruledef
{
    ; -----------------------------------------------------------------------
    ; Control
    ; -----------------------------------------------------------------------

    ; 004: normal exit
    exit => 0b0000100`7 @ 0`9

    ; -----------------------------------------------------------------------
    ; Address register (A) — 24-bit
    ; -----------------------------------------------------------------------

    ; 022 Ai = jk: load 6-bit constant into Ai (single parcel)
    ai {i: u3}, {v: u6}               => 0b0010010`7 @ i`3 @ v`6

    ; 021 Ai = v: load 22-bit constant into Ai (long)
    ai_l {i: u3}, {v: u22}            => 0b0010001`7 @ i`3 @ (v >> 16)`6  @ (v & 0xffff)`16

    ; 030 Ai = Aj + Ak
    aadd {i: u3}, {j: u3}, {k: u3}   => 0b0011000`7 @ i`3 @ j`3 @ k`3

    ; 031 Ai = Aj - Ak
    asub {i: u3}, {j: u3}, {k: u3}   => 0b0011001`7 @ i`3 @ j`3 @ k`3

    ; 032 Ai = Aj * Ak (lower 24 bits)
    amul {i: u3}, {j: u3}, {k: u3}   => 0b0011010`7 @ i`3 @ j`3 @ k`3

    ; 023 Ai = Sj (lower 24 bits)
    a_s {i: u3}, {j: u3}              => 0b0010011`7 @ i`3 @ j`3 @ 0`3

    ; -----------------------------------------------------------------------
    ; Scalar register (S) — 64-bit
    ; -----------------------------------------------------------------------

    ; 040 Si = v: load 22-bit constant into Si (long)
    si {i: u3}, {v: u22}              => 0b0100000`7 @ i`3 @ (v >> 16)`6  @ (v & 0xffff)`16

    ; 043 Si = 0
    sclr {i: u3}                      => 0b0100011`7 @ i`3 @ 0`6

    ; 071 Si = Ak (zero-extend from 24-bit)
    s_a {i: u3}, {k: u3}              => 0b0111001`7 @ i`3 @ 0b000`3 @ k`3

    ; 060 Si = Sj + Sk
    sadd {i: u3}, {j: u3}, {k: u3}   => 0b0110000`7 @ i`3 @ j`3 @ k`3

    ; 061 Si = Sj - Sk
    ssub {i: u3}, {j: u3}, {k: u3}   => 0b0110001`7 @ i`3 @ j`3 @ k`3

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
    loada {i: u3}, {h: u3}, {addr: u22}  => 0b1000`4 @ h`3 @ i`3 @ (addr >> 16)`6 @ (addr & 0xffff)`16

    ; 0o110|h  mem[Ah + addr] = Ai
    storea {i: u3}, {h: u3}, {addr: u22} => 0b1001`4 @ h`3 @ i`3 @ (addr >> 16)`6 @ (addr & 0xffff)`16

    ; 0o120|h  Si = mem[Ah + addr]
    loads {i: u3}, {h: u3}, {addr: u22}  => 0b1010`4 @ h`3 @ i`3 @ (addr >> 16)`6 @ (addr & 0xffff)`16

    ; 0o130|h  mem[Ah + addr] = Si
    stores {i: u3}, {h: u3}, {addr: u22} => 0b1011`4 @ h`3 @ i`3 @ (addr >> 16)`6 @ (addr & 0xffff)`16
}
