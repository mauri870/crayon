use std::fmt;

use crate::ibuffer::InstructionBuffers;
use crate::instr::decode;
use crate::memory::Memory;

// 24-bit address register mask
const ADDR_MASK: u32 = 0x00FF_FFFF;

// Maximum vector length
pub const VL_MAX: u8 = 64;

#[derive(Debug)]
pub struct Registers {
    // A0-A7: primary address registers (24-bit)
    pub a: [u32; 8],
    // S0-S7: primary scalar registers (64-bit)
    pub s: [u64; 8],
    // V0-V7: vector registers, each holds 64 64-bit elements
    pub v: [[u64; 64]; 8],

    // B00-B77: intermediate address registers (64 x 24-bit)
    pub b: [u32; 64],
    // T00-T77: intermediate scalar registers (64 x 64-bit)
    pub t: [u64; 64],

    // VL: vector length (0-64)
    pub vl: u8,
    // VM: vector mask (bit i guards element i)
    pub vm: u64,

    // P: parcel counter (word address in bits 23:2, parcel index in bits 1:0)
    pub p: u32,
    // Real-time clock counter
    pub rtc: u64,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            a: [0; 8],
            s: [0; 8],
            v: [[0; 64]; 8],
            b: [0; 64],
            t: [0; 64],
            vl: 0,
            vm: 0,
            p: 0,
            rtc: 0,
        }
    }

    pub fn write_a(&mut self, i: usize, val: u32) {
        self.a[i] = val & ADDR_MASK;
    }

    pub fn write_b(&mut self, i: usize, val: u32) {
        self.b[i] = val & ADDR_MASK;
    }

    pub fn write_vl(&mut self, val: u8) {
        self.vl = val.min(VL_MAX);
    }
}

impl Default for Registers {
    fn default() -> Self {
        Self::new()
    }
}

// Reasons execution stops.
#[derive(Debug, PartialEq)]
pub enum Trap {
    // Normal exit instruction (004xxx).
    NormalExit,
    // Error exit instruction (000xxx).
    ErrorExit,
    // Opcode not yet implemented.
    Unimplemented(u8),
}

impl fmt::Display for Trap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Trap::NormalExit => write!(f, "normal exit"),
            Trap::ErrorExit => write!(f, "error exit"),
            Trap::Unimplemented(op) => write!(f, "unimplemented opcode {:03o}", op),
        }
    }
}

pub struct Cpu {
    pub regs: Registers,
    pub mem: Memory,
    ibufs: InstructionBuffers,
}

impl Cpu {
    pub fn new(mem: Memory) -> Self {
        Self {
            regs: Registers::new(),
            mem,
            ibufs: InstructionBuffers::new(),
        }
    }

    // Execute one instruction. Returns Err(Trap) when execution should stop.
    pub fn step(&mut self) -> Result<(), Trap> {
        let p = self.regs.p;
        let raw = self.ibufs.fetch(p, &self.mem);
        let next = self.ibufs.fetch(p + 1, &self.mem);
        let d = decode(raw, next);

        self.regs.p += if d.long { 2 } else { 1 };

        self.execute(d)
    }

    fn execute(&mut self, d: crate::instr::Decoded) -> Result<(), Trap> {
        let i = d.i as usize;
        let j = d.j as usize;
        let k = d.k as usize;

        match d.opcode {
            // --- Control ---
            0o000 => return Err(Trap::ErrorExit),
            0o004 => return Err(Trap::NormalExit),

            // VL: transmit (Ak) to VL register
            0o020 => self.regs.write_vl(self.regs.a[k] as u8),
            // VM: transmit (Sj) to VM
            0o033 => self.regs.vm = self.regs.s[j],
            // VM: clear VM
            0o034 => self.regs.vm = 0,

            // --- Address register transmit ---
            // Ai = sign-extended 22-bit constant (addr22)
            0o021 => self.regs.write_a(i, d.addr22),
            // Ai = jk (6-bit constant from same parcel)
            0o022 => self.regs.write_a(i, d.jk as u32),
            // Ai = Sj (lower 24 bits)
            0o023 => self.regs.write_a(i, self.regs.s[j] as u32),
            // Ai = Bjk
            0o024 => self.regs.write_a(i, self.regs.b[j << 3 | k]),
            // Bjk = Ai
            0o025 => self.regs.write_b(j << 3 | k, self.regs.a[i]),

            // --- Address integer arithmetic ---
            // Ai = Aj + Ak  (24-bit, wraps)
            0o030 => self.regs.write_a(i, self.regs.a[j].wrapping_add(self.regs.a[k])),
            // Ai = Aj - Ak
            0o031 => self.regs.write_a(i, self.regs.a[j].wrapping_sub(self.regs.a[k])),
            // Ai = Aj * Ak  (lower 24 bits of product)
            0o032 => self.regs.write_a(i, self.regs.a[j].wrapping_mul(self.regs.a[k])),

            _ => return Err(Trap::Unimplemented(d.opcode)),
        }

        Ok(())
    }
}

impl fmt::Display for Registers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "A  {:06X} {:06X} {:06X} {:06X} {:06X} {:06X} {:06X} {:06X}",
            self.a[0], self.a[1], self.a[2], self.a[3],
            self.a[4], self.a[5], self.a[6], self.a[7])?;
        writeln!(f, "S  {:016X} {:016X} {:016X} {:016X}",
            self.s[0], self.s[1], self.s[2], self.s[3])?;
        writeln!(f, "   {:016X} {:016X} {:016X} {:016X}",
            self.s[4], self.s[5], self.s[6], self.s[7])?;
        writeln!(f, "V  (V0-V7, {} element{} each)",
            self.vl, if self.vl == 1 { "" } else { "s" })?;
        for i in 0..8 {
            write!(f, "   V{i}")?;
            for j in 0..self.vl as usize {
                write!(f, " {:016X}", self.v[i][j])?;
            }
            writeln!(f)?;
        }
        writeln!(f, "VL {:02X}  VM {:016X}  P {:06X}  RTC {:016X}",
            self.vl, self.vm, self.p, self.rtc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_a_masks_to_24_bits() {
        let mut regs = Registers::new();
        regs.write_a(0, 0xFF_ABCD_EF);
        assert_eq!(regs.a[0], 0x00_ABCD_EF);
    }

    #[test]
    fn write_b_masks_to_24_bits() {
        let mut regs = Registers::new();
        regs.write_b(0, 0xFF_123456);
        assert_eq!(regs.b[0], 0x00_123456);
    }

    #[test]
    fn write_vl_caps_at_64() {
        let mut regs = Registers::new();
        regs.write_vl(200);
        assert_eq!(regs.vl, 64);
    }

    #[test]
    fn step_normal_exit() {
        // opcode 004xxx = normal exit: g=0(0000b), h=4(100b) -> 0000_100_xxx = 0x0800...
        // wait: g bits 15:12, h bits 11:9. g=0,h=4 -> 0x0800?
        // 0o004 = 4: g=0(top4), h=4(bot3) -> bits11:9=100 -> 0x0800
        let mut mem = Memory::new();
        mem.write(0, 0x0800_0000_0000_0000);
        let mut cpu = Cpu::new(mem);
        assert_eq!(cpu.step(), Err(Trap::NormalExit));
    }

    #[test]
    fn step_advances_p() {
        // normal exit is 16-bit, so P advances by 1
        let mut mem = Memory::new();
        mem.write(0, 0x0800_0000_0000_0000);
        let mut cpu = Cpu::new(mem);
        let _ = cpu.step();
        assert_eq!(cpu.regs.p, 1);
    }
}
