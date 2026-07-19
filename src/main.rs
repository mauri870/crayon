use std::process;

use crayon::assembler::assemble;
use crayon::cpu::{Cpu, Trap};
use crayon::memory::Memory;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: {} <program.asm|program.bin>", args[0]);
        eprintln!();
        eprintln!("  .asm  assembled with the cray-1 ruleset, then run");
        eprintln!("  .bin  loaded as a flat big-endian binary, then run");
        process::exit(1);
    }

    let path = &args[1];

    let bytes = if path.ends_with(".asm") {
        let src = std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("error: {path}: {e}");
            process::exit(1);
        });
        assemble(&src).unwrap_or_else(|e| {
            eprint!("{e}");
            process::exit(1);
        })
    } else {
        std::fs::read(path).unwrap_or_else(|e| {
            eprintln!("error: {path}: {e}");
            process::exit(1);
        })
    };

    let mut mem = Memory::new();
    mem.load_program(&bytes);

    let mut cpu = Cpu::new(mem);

    loop {
        match cpu.step() {
            Ok(()) => {}
            Err(Trap::NormalExit) => {
                println!("halted: normal exit at P={:06X}  cycles={}", cpu.regs.p, cpu.cycle);
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
