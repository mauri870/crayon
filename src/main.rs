use crayon::cpu::{Cpu, Trap};
use crayon::memory::Memory;

fn main() {
    // Program: store S1 to memory, load it back into S2, then exit.
    //
    //   P=0: A1 = 50           (base address, fits in 6-bit jk)
    //   P=1: A2 = 0xABCD       (value, via 22-bit constant → 021)  [long]
    //   P=3: S1 = A2
    //   P=4: mem[A1+0] = S1    (store, Ah=A1, opcode 131)  [long]
    //   P=6: S2 = mem[A1+0]    (load,  Ah=A1, opcode 121)  [long]
    //   P=8: exit
    fn p(op: u8, i: u8, j: u8, k: u8) -> [u8; 2] {
        let g = op >> 3;
        let h = op & 0x7;
        let word = ((g as u16) << 12) | ((h as u16) << 9)
                 | ((i as u16) << 6) | ((j as u16) << 3) | (k as u16);
        word.to_be_bytes()
    }
    fn pjk(op: u8, i: u8, jk: u8) -> [u8; 2] { p(op, i, (jk >> 3) & 0x7, jk & 0x7) }
    fn mem_instr(op: u8, i: u8, addr: u32) -> [u8; 4] {
        let jk = ((addr >> 16) & 0x3F) as u8;
        let [p0, p0b] = p(op, i, (jk >> 3) & 0x7, jk & 0x7);
        let [p1a, p1b] = (addr as u16).to_be_bytes();
        [p0, p0b, p1a, p1b]
    }
    // 32-bit constant load into A register: opcode 021, addr22 carries the value.
    fn load_a_const(i: u8, val: u32) -> [u8; 4] {
        let jk = ((val >> 16) & 0x3F) as u8;
        let [p0, p0b] = p(0o21, i, (jk >> 3) & 0x7, jk & 0x7);
        let [p1a, p1b] = (val as u16).to_be_bytes();
        [p0, p0b, p1a, p1b]
    }

    let [a0, a1]            = pjk(0o22, 1, 50);      // A1 = 50         (P=0)
    let [b0, b1, b2, b3]    = load_a_const(2, 0xABCD); // A2 = 0xABCD  (P=1,2)
    let [c0, c1]            = p(0o071, 1, 0, 2);      // S1 = A2         (P=3)
    let [d0, d1, d2, d3]    = mem_instr(0o131, 1, 0); // mem[A1+0] = S1  (P=4,5)
    let [e0, e1, e2, e3]    = mem_instr(0o121, 2, 0); // S2 = mem[A1+0]  (P=6,7)
    let [x0, x1]            = p(0o04, 0, 0, 0);       // exit             (P=8)

    let program: &[u8] = &[
        a0, a1,
        b0, b1, b2, b3,
        c0, c1,
        d0, d1, d2, d3,
        e0, e1, e2, e3,
        x0, x1,
        0, 0, 0, 0, 0, 0,  // pad to word boundary
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

    println!("mem[50] = {:016X}  (stored by S1, reloaded into S2)", cpu.mem.read(50));
    print!("{}", cpu.regs);
}
