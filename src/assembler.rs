// Cray-1 instruction ruleset for customasm, embedded at compile time.
const RULESET: &str = include_str!("../cray1.asm");

/// Assemble Cray-1 source code into a flat binary.
///
/// Returns the assembled bytes on success, or an error string containing
/// the assembler diagnostics.
pub fn assemble(src: &str) -> Result<Vec<u8>, String> {
    let combined = format!("{}\n{}", RULESET, src);
    let filename = "input.asm";

    let mut report = customasm::diagn::Report::new();
    let mut fileserver = customasm::util::FileServerMock::new();
    fileserver.add(filename, combined.as_str());

    let opts = customasm::asm::AssemblyOptions::new();
    let assembly = customasm::asm::assemble(
        &mut report,
        &opts,
        &mut fileserver,
        &[filename],
    );

    if report.has_errors() {
        let mut buf = Vec::new();
        report.print_all(&mut buf, &fileserver, false);
        return Err(String::from_utf8_lossy(&buf).into_owned());
    }

    Ok(assembly.output
        .map(|o| o.format_binary(&mut report))
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::{Cpu, Trap};
    use crate::memory::Memory;

    fn run(src: &str) -> Cpu {
        let bytes = assemble(src).expect("assembly failed");
        let mut mem = Memory::new();
        mem.load_program(&bytes);
        let mut cpu = Cpu::new(mem);
        while cpu.step().is_ok() {}
        cpu
    }

    #[test]
    fn assemble_exit() {
        let cpu = run("exit");
        assert_eq!(cpu.regs.p, 1);
    }

    #[test]
    fn assemble_load_constants() {
        let cpu = run("ai 1, 5\n ai 2, 3\n exit");
        assert_eq!(cpu.regs.a[1], 5);
        assert_eq!(cpu.regs.a[2], 3);
    }

    #[test]
    fn assemble_error_returns_err() {
        assert!(assemble("not_an_instruction").is_err());
    }
}
