use crayon::cpu::{Cpu, Trap};
use crayon::memory::Memory;

fn main() {
    // Program:
    //   A1 = 10
    //   S1 = A1            (zero-extend)
    //   A2 = 32
    //   S2 = A2
    //   S3 = S1 + S2       -> 42
    //   S4 = S1 & S2       -> 0 (10 & 32 = 0)
    //   S5 = S1 ^ S2       -> 42 (10 ^ 32 = 42)
    //   exit
    fn p(op: u8, i: u8, j: u8, k: u8) -> [u8; 2] {
        let g = op >> 3;
        let h = op & 0x7;
        let word = ((g as u16) << 12) | ((h as u16) << 9)
                 | ((i as u16) << 6) | ((j as u16) << 3) | (k as u16);
        word.to_be_bytes()
    }
    fn pjk(op: u8, i: u8, jk: u8) -> [u8; 2] { p(op, i, (jk >> 3) & 0x7, jk & 0x7) }

    let program: &[u8] = &[
        pjk(0o22, 1, 10)[0], pjk(0o22, 1, 10)[1],  // A1 = 10
        p(0o071, 1, 0, 1)[0], p(0o071, 1, 0, 1)[1], // S1 = A1
        pjk(0o22, 2, 32)[0], pjk(0o22, 2, 32)[1],   // A2 = 32
        p(0o071, 2, 0, 2)[0], p(0o071, 2, 0, 2)[1], // S2 = A2
        p(0o060, 3, 1, 2)[0], p(0o060, 3, 1, 2)[1], // S3 = S1 + S2
        p(0o044, 4, 1, 2)[0], p(0o044, 4, 1, 2)[1], // S4 = S1 & S2
        p(0o046, 5, 1, 2)[0], p(0o046, 5, 1, 2)[1], // S5 = S1 ^ S2
        p(0o04, 0, 0, 0)[0], p(0o04, 0, 0, 0)[1],   // exit
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
