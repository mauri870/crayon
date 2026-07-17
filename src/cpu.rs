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

            // --- Scalar transmit ---
            // Si = 22-bit constant (zero-extended)
            0o040 => self.regs.s[i] = d.addr22 as u64,
            // Si = 0
            0o043 => self.regs.s[i] = 0,
            // Si = Ak (zero-extended from 24-bit)
            0o071 if d.j == 0 => self.regs.s[i] = self.regs.a[k] as u64,
            // Si = Ak (sign-extended from 24-bit)
            0o071 if d.j == 1 => self.regs.s[i] = self.regs.a[k] as i32 as i64 as u64,
            // Si = Tjk
            0o074 => self.regs.s[i] = self.regs.t[j << 3 | k],
            // Tjk = Si
            0o075 => self.regs.t[j << 3 | k] = self.regs.s[i],

            // --- Scalar integer arithmetic ---
            // Si = Sj + Sk
            0o060 => self.regs.s[i] = self.regs.s[j].wrapping_add(self.regs.s[k]),
            // Si = Sj - Sk
            0o061 => self.regs.s[i] = self.regs.s[j].wrapping_sub(self.regs.s[k]),

            // --- Scalar logical ---
            // Si = Sj & Sk
            0o044 => self.regs.s[i] = self.regs.s[j] & self.regs.s[k],
            // Si = Sj & ~Sk
            0o045 => self.regs.s[i] = self.regs.s[j] & !self.regs.s[k],
            // Si = Sj ^ Sk
            0o046 => self.regs.s[i] = self.regs.s[j] ^ self.regs.s[k],
            // Si = ~(Sj ^ Sk)  (logical equivalence / XNOR)
            0o047 => self.regs.s[i] = !(self.regs.s[j] ^ self.regs.s[k]),
            // Si = (Si & ~Sk) | (Sj & Sk)  (merge: select Sj where Sk=1, Si where Sk=0)
            0o050 => self.regs.s[i] = (self.regs.s[i] & !self.regs.s[k]) | (self.regs.s[j] & self.regs.s[k]),
            // Si = (Si & ~mask) | (Sj & mask) where mask = sign bit of Sj broadcast
            0o051 => {
                let mask = ((self.regs.s[j] as i64) >> 63) as u64;
                self.regs.s[i] = (self.regs.s[i] & !mask) | (self.regs.s[j] & mask);
            }

            // --- Scalar shifts ---
            // S0 = Si << jk
            0o052 => self.regs.s[0] = self.regs.s[i] << d.jk,
            // S0 = Si >> (64 - jk)
            0o053 => self.regs.s[0] = if d.jk == 0 { 0 } else { self.regs.s[i] >> (64 - d.jk) },
            // Si = Si << jk
            0o054 => self.regs.s[i] <<= d.jk,
            // Si = Si >> (64 - jk)
            0o055 => self.regs.s[i] = if d.jk == 0 { 0 } else { self.regs.s[i] >> (64 - d.jk) },
            // Si = high 64 bits of (Si:Sj) << Ak  (double-length left shift)
            0o056 => {
                let count = (self.regs.a[k] & 0x7F) as u32;
                self.regs.s[i] = if count == 0 { self.regs.s[i] }
                    else if count >= 64 { self.regs.s[j] << (count - 64) }
                    else { (self.regs.s[i] << count) | (self.regs.s[j] >> (64 - count)) };
            }
            // Si = low 64 bits of (Sj:Si) >> Ak  (double-length right shift)
            0o057 => {
                let count = (self.regs.a[k] & 0x7F) as u32;
                self.regs.s[i] = if count == 0 { self.regs.s[i] }
                    else if count >= 64 { self.regs.s[j] >> (count - 64) }
                    else { (self.regs.s[j] << (64 - count)) | (self.regs.s[i] >> count) };
            }

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

    // Encode a 16-bit parcel from octal opcode and register fields.
    fn parcel(opcode: u8, i: u8, j: u8, k: u8) -> [u8; 2] {
        let g = opcode >> 3;
        let h = opcode & 0x7;
        let word = ((g as u16) << 12) | ((h as u16) << 9)
                 | ((i as u16) << 6) | ((j as u16) << 3) | (k as u16);
        word.to_be_bytes()
    }

    // Encode a parcel where j:k form a 6-bit constant (shift/mask/small-constant instructions).
    fn parcel_jk(opcode: u8, i: u8, jk: u8) -> [u8; 2] {
        parcel(opcode, i, (jk >> 3) & 0x7, jk & 0x7)
    }

    fn cpu_with_program(bytes: &[u8]) -> Cpu {
        let mut mem = Memory::new();
        mem.load_program(bytes);
        Cpu::new(mem)
    }

    #[test]
    fn addr_transmit_constant() {
        let [b0, b1] = parcel_jk(0o22, 1, 5); // A1 = 5
        let [e0, e1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[b0, b1, e0, e1, 0, 0, 0, 0]);
        cpu.step().unwrap();
        assert_eq!(cpu.regs.a[1], 5);
    }

    #[test]
    fn addr_add() {
        let [a0, a1] = parcel_jk(0o22, 1, 3); // A1 = 3
        let [b0, b1] = parcel_jk(0o22, 2, 5); // A2 = 5
        let [c0, c1] = parcel(0o30, 3, 1, 2); // A3 = A1 + A2
        let [e0, e1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, b0, b1, c0, c1, e0, e1]);
        for _ in 0..3 { cpu.step().unwrap(); }
        assert_eq!(cpu.regs.a[3], 8);
    }

    #[test]
    fn addr_sub() {
        let [a0, a1] = parcel_jk(0o22, 1, 10); // A1 = 10
        let [b0, b1] = parcel_jk(0o22, 2, 3);  // A2 = 3
        let [c0, c1] = parcel(0o31, 3, 1, 2);  // A3 = A1 - A2
        let [e0, e1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, b0, b1, c0, c1, e0, e1]);
        for _ in 0..3 { cpu.step().unwrap(); }
        assert_eq!(cpu.regs.a[3], 7);
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

    #[test]
    fn scalar_add() {
        // S1 = 10, S2 = 32; S3 = S1 + S2 = 42
        let [a0, a1] = parcel_jk(0o22, 1, 10); // A1 = 10
        let [b0, b1] = parcel(0o071, 1, 0, 1); // S1 = A1 (zero-extend)
        let [c0, c1] = parcel_jk(0o22, 2, 32); // A2 = 32
        let [d0, d1] = parcel(0o071, 2, 0, 2); // S2 = A2
        let [e0, e1] = parcel(0o060, 3, 1, 2); // S3 = S1 + S2
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, b0, b1, c0, c1, d0, d1, e0, e1, x0, x1, 0, 0, 0, 0]);
        for _ in 0..5 { cpu.step().unwrap(); }
        assert_eq!(cpu.regs.s[3], 42);
    }

    #[test]
    fn scalar_sub() {
        let [a0, a1] = parcel_jk(0o22, 1, 20); // A1 = 20
        let [b0, b1] = parcel(0o071, 1, 0, 1); // S1 = A1
        let [c0, c1] = parcel_jk(0o22, 2, 7);  // A2 = 7
        let [d0, d1] = parcel(0o071, 2, 0, 2); // S2 = A2
        let [e0, e1] = parcel(0o061, 3, 1, 2); // S3 = S1 - S2
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, b0, b1, c0, c1, d0, d1, e0, e1, x0, x1, 0, 0, 0, 0]);
        for _ in 0..5 { cpu.step().unwrap(); }
        assert_eq!(cpu.regs.s[3], 13);
    }

    #[test]
    fn scalar_and() {
        // S1 = 0xFF, S2 = 0x0F; S3 = S1 & S2 = 0x0F
        let [a0, a1] = parcel_jk(0o22, 1, 0o77); // A1 = 63 (0x3F max in 6 bits)
        let [b0, b1] = parcel(0o071, 1, 0, 1);
        let [c0, c1] = parcel_jk(0o22, 2, 0o17); // A2 = 15
        let [d0, d1] = parcel(0o071, 2, 0, 2);
        let [e0, e1] = parcel(0o044, 3, 1, 2);   // S3 = S1 & S2
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, b0, b1, c0, c1, d0, d1, e0, e1, x0, x1, 0, 0, 0, 0]);
        for _ in 0..5 { cpu.step().unwrap(); }
        assert_eq!(cpu.regs.s[3], 0x3F & 0x0F);
    }

    #[test]
    fn scalar_xor() {
        let [a0, a1] = parcel_jk(0o22, 1, 0o77); // A1 = 63
        let [b0, b1] = parcel(0o071, 1, 0, 1);
        let [c0, c1] = parcel_jk(0o22, 2, 0o17); // A2 = 15
        let [d0, d1] = parcel(0o071, 2, 0, 2);
        let [e0, e1] = parcel(0o046, 3, 1, 2);   // S3 = S1 ^ S2
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, b0, b1, c0, c1, d0, d1, e0, e1, x0, x1, 0, 0, 0, 0]);
        for _ in 0..5 { cpu.step().unwrap(); }
        assert_eq!(cpu.regs.s[3], 0x3F ^ 0x0F);
    }

    #[test]
    fn scalar_shift_left() {
        // S1 = 1; S0 = S1 << 4 = 16 (opcode 052: S0 = Si << jk)
        let [a0, a1] = parcel_jk(0o22, 1, 1); // A1 = 1
        let [b0, b1] = parcel(0o071, 1, 0, 1); // S1 = A1
        let [c0, c1] = parcel_jk(0o052, 1, 4); // S0 = S1 << 4
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, b0, b1, c0, c1, x0, x1]);
        for _ in 0..3 { cpu.step().unwrap(); }
        assert_eq!(cpu.regs.s[0], 16);
    }

    #[test]
    fn scalar_zero() {
        // Si = 0 (opcode 043)
        let [a0, a1] = parcel_jk(0o22, 1, 63); // A1 = 63
        let [b0, b1] = parcel(0o071, 1, 0, 1); // S1 = A1
        let [c0, c1] = parcel(0o043, 1, 0, 0); // S1 = 0
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, b0, b1, c0, c1, x0, x1]);
        for _ in 0..3 { cpu.step().unwrap(); }
        assert_eq!(cpu.regs.s[1], 0);
    }
}
