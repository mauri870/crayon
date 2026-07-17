use crayon::cpu::{Cpu, Trap};
use crayon::memory::Memory;

fn main() {
    // Program:
    // A1 = 4, A2 = A1 + A1, exit.
    use crayon::instr;
    let _ = instr::decode; // ensure module is used
    fn p(op: u8, i: u8, j: u8, k: u8) -> [u8; 2] {
        let w = (((op >> 3) as u16) << 12) | (((op & 7) as u16) << 9)
              | ((i as u16) << 6) | ((j as u16) << 3) | (k as u16);
        w.to_be_bytes()
    }
    let [a0, a1] = p(0o22, 1, 0, 4); // A1 = 4  (jk=4 packed as j=0,k=4)
    let [b0, b1] = p(0o30, 2, 1, 1); // A2 = A1 + A1
    let [e0, e1] = p(0o04, 0, 0, 0); // exit
    let program: &[u8] = &[a0, a1, b0, b1, e0, e1, 0x00, 0x00];

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
