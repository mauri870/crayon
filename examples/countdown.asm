; countdown.asm: count A0 from 5 down to 0
; Expect: A0=0 A1=1 cycles=18

    ai a0, 5         ; A0 = 5
    ai a1, 1         ; A1 = 1 (step)
loop:
    asub a0, a0, a1  ; A0 = A0 - A1
    jan loop         ; branch back if A0 ≠ 0
    exit
