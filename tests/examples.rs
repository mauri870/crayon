use crayon::assembler::assemble;
use crayon::cpu::Cpu;
use crayon::memory::Memory;

fn run_example(path: &std::path::Path) -> datatest_stable::Result<()> {
    let src = std::fs::read_to_string(path)?;
    let expects = parse_expects(&src);
    if expects.is_empty() {
        return Ok(());
    }
    let cpu = run_asm(&src);
    let name = path.display().to_string();
    for (reg, val) in &expects {
        check_register(&cpu, reg, *val, &name);
    }
    Ok(())
}

fn run_asm(src: &str) -> Cpu {
    let bytes = assemble(src).expect("assembly failed");
    let mut mem = Memory::new();
    mem.load_program(&bytes);
    let mut cpu = Cpu::new(mem);
    while cpu.step().is_ok() {}
    cpu
}

// Parse `; Expect: REG=VALUE ...` lines from assembly source.
// Values may be decimal or 0x-prefixed hex.
fn parse_expects(src: &str) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    for line in src.lines() {
        if let Some(rest) = line.trim().strip_prefix("; Expect:") {
            for token in rest.split_whitespace() {
                if let Some((name, val)) = token.split_once('=') {
                    let val = if let Some(hex) = val.strip_prefix("0x") {
                        u64::from_str_radix(hex, 16).unwrap_or_else(|_| panic!("bad hex: {val}"))
                    } else {
                        val.parse::<u64>().unwrap_or_else(|_| panic!("bad integer: {val}"))
                    };
                    out.push((name.to_uppercase(), val));
                }
            }
        }
    }
    out
}

fn check_register(cpu: &Cpu, name: &str, expected: u64, file: &str) {
    let actual = match name {
        n if n.len() == 2 && n.starts_with('A') => {
            cpu.regs.a[n[1..].parse::<usize>().unwrap()] as u64
        }
        n if n.len() == 2 && n.starts_with('S') => {
            cpu.regs.s[n[1..].parse::<usize>().unwrap()]
        }
        "VL" => cpu.regs.vl as u64,
        "P" => cpu.regs.p as u64,
        _ => panic!("{file}: unknown register '{name}'"),
    };
    assert_eq!(
        actual, expected,
        "{file}: {name} = {actual:#x}, expected {expected:#x}"
    );
}

datatest_stable::harness!(run_example, "examples", r".*\.asm$");
