use crayon::cpu::Registers;
use crayon::memory::Memory;

fn main() {
    let regs = Registers::new();
    print!("{regs}");

    let mut mem = Memory::new();
    // Load a small pattern so its visible in the dump.
    let program: &[u8] = &[
        0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // word 0
        0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE, // word 1
    ];
    mem.load_program(program);

    println!("\nMemory (words 0-3):");
    for addr in 0..4u32 {
        println!("  [{addr:06X}] {:016X}", mem.read(addr));
    }
}
