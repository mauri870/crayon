use crayon::cpu::Registers;
use crayon::instr::decode;
use crayon::memory::Memory;

fn main() {
    let regs = Registers::new();
    print!("{regs}");

    let mut mem = Memory::new();
    // A tiny hand-encoded program:
    //   parcel 0: 0x2004 opcode=020 k=4 -> VL = A4
    //   parcel 1: 0x6053 opcode=060 i=1, j=2, k=3 -> S1 = S2 + S3
    let program: &[u8] = &[
        0x20, 0x04, 0x60, 0x53, 0x00, 0x00, 0x00, 0x00, // word 0: parcels 0-3
    ];
    mem.load_program(program);

    println!("\nMemory (word 0): {:016X}", mem.read(0));

    println!("\nDecoded parcels:");
    let word = mem.read(0);
    for slot in 0..4 {
        let shift = 48 - slot * 16;
        let p0 = ((word >> shift) & 0xFFFF) as u16;
        let p1 = if slot < 3 { ((word >> (shift - 16)) & 0xFFFF) as u16 } else { 0 };
        let d = decode(p0, p1);
        println!("  parcel {slot}: opcode={:03o} i={} j={} k={} long={}",
            d.opcode, d.i, d.j, d.k, d.long);
    }
}
