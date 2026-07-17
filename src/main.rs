use std::process;

use crayon::cpu::{Cpu, Trap};
use crayon::memory::Memory;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: {} <program.bin>", args[0]);
        eprintln!();
        eprintln!("  Loads a flat binary image at word address 0 and executes it.");
        eprintln!("  The binary must be big-endian: byte 0 maps to bits 63:56 of word 0.");
        process::exit(1);
    }

    let bytes = std::fs::read(&args[1]).unwrap_or_else(|e| {
        eprintln!("error: {}: {}", args[1], e);
        process::exit(1);
    });

    let mut mem = Memory::new();
    mem.load_program(&bytes);

    let mut cpu = Cpu::new(mem);

    loop {
        match cpu.step() {
            Ok(()) => {}
            Err(Trap::NormalExit) => {
                println!("halted: normal exit at P={:06X}", cpu.regs.p);
                break;
            }
            Err(t) => {
                eprintln!("halted: {t} at P={:06X}", cpu.regs.p);
                process::exit(1);
            }
        }
    }

    print!("{}", cpu.regs);
}
