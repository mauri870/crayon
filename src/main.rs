use std::io::{self, BufRead, Write};
use std::process;
use std::time::{Duration, Instant};

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

    let mut step = false;
    let mut watch = false;
    let mut slow_hz: Option<f64> = None;
    let mut path: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--step" => step = true,
            "--watch" => watch = true,
            "--speed" => {
                i += 1;
                let val = args.get(i).map(String::as_str).unwrap_or("");
                match val.parse::<f64>() {
                    Ok(hz) if hz > 0.0 => slow_hz = Some(hz),
                    _ => {
                        eprintln!("error: --speed requires a positive number (instructions/sec)");
                        process::exit(1);
                    }
                }
            }
            arg if !arg.starts_with('-') => {
                if path.is_some() {
                    eprintln!("error: unexpected argument '{arg}'");
                    process::exit(1);
                }
                path = Some(arg.to_string());
            }
            arg => {
                eprintln!("error: unknown flag '{arg}'");
                process::exit(1);
            }
        }
        i += 1;
    }

    let path = path.unwrap_or_else(|| {
        eprintln!("usage: {} [--step] [--watch] [--speed <n>] <program.asm|program.bin>", args[0]);
        eprintln!();
        eprintln!("  .asm          assembled with the cray-1 ruleset, then run");
        eprintln!("  .bin          loaded as a flat big-endian binary, then run");
        eprintln!("  --step        pause after each instruction; Enter to advance, q to quit");
        eprintln!("  --watch       live register display updated after each instruction");
        eprintln!("  --speed <n>      throttle to <n> instructions per second");
        process::exit(1);
    });

    let bytes = if path.ends_with(".asm") {
        let src = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("error: {path}: {e}");
            process::exit(1);
        });
        assemble(&src).unwrap_or_else(|e| {
            eprint!("{e}");
            process::exit(1);
        })
    } else {
        std::fs::read(&path).unwrap_or_else(|e| {
            eprintln!("error: {path}: {e}");
            process::exit(1);
        })
    };

    let mut mem = Memory::new();
    mem.load_program(&bytes);

    let mut cpu = Cpu::new(mem);
    let wall_start = Instant::now();

    let sleep_dur = slow_hz.map(|hz| Duration::from_secs_f64(1.0 / hz));
    let live = step || watch;
    let stdin = io::stdin();

    loop {
        match cpu.step() {
            Ok(()) => {
                if live {
                    print!("\x1b[2J\x1b[H");
                    println!("cycle {}  P={:06X}", cpu.cycle, cpu.regs.p);
                    print!("{}", cpu.regs);
                    if step {
                        print!("> ");
                    }
                    io::stdout().flush().ok();
                }
                if step {
                    let mut line = String::new();
                    stdin.lock().read_line(&mut line).ok();
                    if line.trim() == "q" {
                        process::exit(0);
                    }
                } else if let Some(dur) = sleep_dur {
                    std::thread::sleep(dur);
                }
            }
            Err(Trap::NormalExit) => {
                let wall_ns = wall_start.elapsed().as_nanos() as u64;
                let sim_ns = cpu.cycle * 25 / 2; // cycle * 12.5 ns
                if live {
                    print!("\x1b[2J\x1b[H");
                }
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
