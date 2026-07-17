use crayon::cpu::{Cpu, Trap};
use crayon::memory::Memory;

fn main() {
    // Program:
    //   word 0 parcel 0: 022i04  A1 = 4  (opcode=022, i=1, jk=4)
    //   word 0 parcel 1: 030120  A2 = A1 + A1  (opcode=030, i=2, j=1, k=1? wait j=1,k=0 -> A1+A0)
    //   word 0 parcel 2: 030212  A2 = A1 + A2 is (opcode=030, i=2, j=1, k=2)
    //   word 0 parcel 3: 0x0800  normal exit
    //
    // Let's encode cleanly:
    //   022 i=1 jk=4:      g=2,h=2 -> bits15:12=0010,11:9=010 -> 0x4400 | (1<<6) | 4 = 0x4444?
    //   Let's compute: opcode=0o22=18 -> g=18>>3=2,h=18&7=2
    //                  parcel = (g<<12)|(h<<9)|(i<<6)|jk = (2<<12)|(2<<9)|(1<<6)|4 = 0x1000|0x0400|0x0040|0x0004 = 0x1444
    //   030 i=2 j=1 k=1:  opcode=0o30=24 -> g=3,h=0
    //                  parcel = (3<<12)|(0<<9)|(2<<6)|(1<<3)|1 = 0x3000|0|0x0080|0x0008|0x0001 = 0x3089
    //   004 normal exit:   opcode=0o4=4 -> g=0,h=4
    //                  parcel = (0<<12)|(4<<9) = 0x0800
    let program: &[u8] = &[
        0x14, 0x44, // parcel 0: A1 = 4
        0x30, 0x89, // parcel 1: A2 = A1 + A1
        0x08, 0x00, // parcel 2: normal exit
        0x00, 0x00, // parcel 3: (unused)
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
