use std::process;
use std::time::Instant;

use crayon::assembler::assemble;
use crayon::cpu::{Cpu, Trap};
use crayon::memory::Memory;

fn fmt_ns(ns: u64) -> String {
    if ns < 1_000 {
        format!("{ns}ns")
    } else if ns < 1_000_000 {
        format!("{:.3}µs", ns as f64 / 1_000.0)
    } else if ns < 1_000_000_000 {
        format!("{:.3}ms", ns as f64 / 1_000_000.0)
    } else {
        format!("{:.3}s", ns as f64 / 1_000_000_000.0)
    }
}

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
    let wall_start = Instant::now();

    loop {
        match cpu.step() {
            Ok(()) => {}
            Err(Trap::NormalExit) => {
                let wall_ns = wall_start.elapsed().as_nanos() as u64;
                let sim_ns = cpu.cycle * 25 / 2; // cycle * 12.5 ns
                println!(
                    "halted: normal exit at P={:06X}  cycles={}  sim={}  wall={}",
                    cpu.regs.p,
                    cpu.cycle,
                    fmt_ns(sim_ns),
                    fmt_ns(wall_ns),
                );
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
