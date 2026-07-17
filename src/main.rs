use crayon::cpu::{Cpu, Trap};
use crayon::memory::Memory;

fn main() {
    // Program: countdown loop.A0 counts from 5 down to 0.
    //
    //   P=0: A0 = 5
    //   P=1: A1 = 1
    //   P=2: A0 = A0 - A1        (loop body)
    //   P=3: JAN -> P=2           (jump back if A0 ≠ 0)
    //   P=4: <second parcel: target = 2>
    //   P=5: normal exit
    fn p(op: u8, i: u8, j: u8, k: u8) -> [u8; 2] {
        let g = op >> 3;
        let h = op & 0x7;
        let word = ((g as u16) << 12) | ((h as u16) << 9)
                 | ((i as u16) << 6) | ((j as u16) << 3) | (k as u16);
        word.to_be_bytes()
    }
    fn pjk(op: u8, i: u8, jk: u8) -> [u8; 2] { p(op, i, (jk >> 3) & 0x7, jk & 0x7) }
    fn p1(val: u16) -> [u8; 2] { val.to_be_bytes() }

    let [a0, a1]     = pjk(0o22, 0, 5);   // A0 = 5       (P=0)
    let [b0, b1]     = pjk(0o22, 1, 1);   // A1 = 1       (P=1)
    let [dec0, dec1] = p(0o31, 0, 0, 1);  // A0 = A0 - A1 (P=2)
    let [jan0, jan1] = p(0o11, 0, 0, 0);  // JAN          (P=3, first parcel)
    let [tgt0, tgt1] = p1(2);             //               (P=4, second parcel: target=2)
    let [ex0, ex1]   = p(0o04, 0, 0, 0);  // normal exit  (P=5)

    let program: &[u8] = &[
        a0, a1, b0, b1, dec0, dec1, jan0, jan1,  // word 0 (P=0..3)
        tgt0, tgt1, ex0, ex1, 0, 0, 0, 0,         // word 1 (P=4..7)
    ];

    let mut mem = Memory::new();
    mem.load_program(program);

    let mut cpu = Cpu::new(mem);

    loop {
        match cpu.step() {
            Ok(()) => {}
            Err(Trap::NormalExit) => {
                println!("halted: normal exit at P={:06X}", cpu.regs.p);
                break;
            }
            Err(t) => {
                println!("halted: {t} at P={:06X}", cpu.regs.p);
                break;
            }
        }
    }

    print!("{}", cpu.regs);
}
