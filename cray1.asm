; Cray-1 instruction set ruleset for customasm.
;
; Each instruction is one 16-bit parcel (single) or two parcels (long, marked L).
; Parcels are big-endian. Four parcels fit in one 64-bit memory word.
; The 7-bit opcode is g(4):h(3) occupying bits 15:9 of the first parcel.
;
; Assemble:  customasm examples/hello.asm -f binary -o hello.bin
; Run:       crayon hello.bin

#bankdef default
{
    #bits 8
    #addr 0
    #size 0x800000  ; 8M bytes = 1M 64-bit words
    #outp 0
}

#ruledef
{
    ; --- Control ---

    ; 004: normal exit
    exit => 0x08`8 @ 0x00`8

    ; --- Address register transmit (single parcel) ---

    ; 022 Ai = jk: load 6-bit constant into address register Ai
    ; parcel bits: g(0010) h(010) i(3) jk(6)
    ai {i: u3}, {v: u6} => 0b0010010`7 @ i`3 @ v`6
}
