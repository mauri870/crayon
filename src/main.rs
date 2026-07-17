use crayon::cpu::Registers;
use crayon::ibuffer::InstructionBuffers;
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
        0x20, 0x04, 0x60, 0x53, 0x00, 0x00, 0x00, 0x00,
    ];
    mem.load_program(program);

    let mut ibufs = InstructionBuffers::new();

    println!("\nDecoded parcels (via instruction buffers):");
    let mut p = 0u32;
    while p < 4 {
        let raw = ibufs.fetch(p, &mem);
        let next = ibufs.fetch(p + 1, &mem);
        let d = decode(raw, next);
        println!("  P={p}: opcode={:03o} i={} j={} k={} long={}", d.opcode, d.i, d.j, d.k, d.long);
        p += if d.long { 2 } else { 1 };
    }
}
