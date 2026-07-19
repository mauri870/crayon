use std::fmt;

use crate::fp;
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
    pub cycle: u64,
    ibufs: InstructionBuffers,
    // Per address register: earliest cycle when the result is available.
    ar_ready_at: [u64; 8],
    // Per scalar register: earliest cycle when the result is available.
    sr_ready_at: [u64; 8],
}

impl Cpu {
    pub fn new(mem: Memory) -> Self {
        Self {
            regs: Registers::new(),
            mem,
            cycle: 0,
            ibufs: InstructionBuffers::new(),
            ar_ready_at: [0; 8],
            sr_ready_at: [0; 8],
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

    // Advances cycle past all source-register readiness times, issues the instruction,
    // and returns the cycle at which it issued.
    fn issue(&mut self, srcs: &[u64]) -> u64 {
        self.cycle = srcs.iter().copied().fold(self.cycle, u64::max);
        let issued_at = self.cycle;
        self.cycle += 1;
        issued_at
    }

    fn execute(&mut self, d: crate::instr::Decoded) -> Result<(), Trap> {
        let i = d.i as usize;
        let j = d.j as usize;
        let k = d.k as usize;

        let h = (d.opcode & 7) as usize; // low 3 bits: base A register for memory opcodes
        let srcs: [u64; 2] = match d.opcode {
            0o010..=0o013                => [self.ar_ready_at[0], 0],
            0o020                        => [self.ar_ready_at[k], 0],
            0o025                        => [self.ar_ready_at[i], 0],
            0o030 | 0o031 | 0o032        => [self.ar_ready_at[j], self.ar_ready_at[k]],
            0o044..=0o047                => [self.sr_ready_at[j], self.sr_ready_at[k]],
            0o050                        => [self.sr_ready_at[i].max(self.sr_ready_at[j]), self.sr_ready_at[k]],
            0o051                        => [self.sr_ready_at[i], self.sr_ready_at[j]],
            0o052 | 0o053                => [self.sr_ready_at[i], 0],
            0o054 | 0o055                => [self.sr_ready_at[i], 0],
            0o056 | 0o057                => [self.sr_ready_at[i].max(self.sr_ready_at[j]), self.ar_ready_at[k]],
            0o060 | 0o061                => [self.sr_ready_at[j], self.sr_ready_at[k]],
            0o062 | 0o063                => [self.sr_ready_at[j], self.sr_ready_at[k]],
            0o064..=0o067                => [self.sr_ready_at[j], self.sr_ready_at[k]],
            0o070                        => [self.sr_ready_at[j], 0],
            0o071 if j <= 2              => [self.ar_ready_at[k], 0],
            0o075                        => [self.sr_ready_at[i], 0],
            0o077                        => [self.sr_ready_at[j], self.ar_ready_at[k]],
            0o100..=0o107                => [self.ar_ready_at[h], 0],
            0o110..=0o117                => [self.ar_ready_at[h], self.ar_ready_at[i]],
            0o120..=0o127                => [self.ar_ready_at[h], 0],
            0o130..=0o137                => [self.ar_ready_at[h], self.sr_ready_at[i]],
            0o150..=0o153                => [self.ar_ready_at[k], 0],
            0o176                        => [self.ar_ready_at[0], if k != 0 { self.ar_ready_at[k] } else { 0 }],
            0o177                        => [self.ar_ready_at[0], if k != 0 { self.ar_ready_at[k] } else { 0 }],
            _                            => [0, 0],
        };
        let issued_at = self.issue(&srcs);

        match d.opcode {
            // --- Control ---
            0o000 => return Err(Trap::ErrorExit),
            0o004 => return Err(Trap::NormalExit),

            // --- Branches and jumps ---
            // J Bjk: unconditional jump to parcel address stored in B register jk (16-bit)
            0o005 if d.i == 0 => self.regs.p = self.regs.b[j << 3 | k],
            // J exp: unconditional jump to 25-bit parcel address exp (32-bit)
            0o005 | 0o006 => self.regs.p = d.addr25,
            // R exp: return jump — save return address in B00, then jump to exp
            0o007 => {
                self.regs.write_b(0, self.regs.p);
                self.regs.p = d.addr25;
            }
            // Conditional branches on A0: JAZ, JAN, JAP, JAM
            0o010 => if self.regs.a[0] == 0                              { self.regs.p = d.addr25; }
            0o011 => if self.regs.a[0] != 0                              { self.regs.p = d.addr25; }
            0o012 => if self.regs.a[0] & (1 << 23) == 0 && self.regs.a[0] != 0 { self.regs.p = d.addr25; }
            0o013 => if self.regs.a[0] & (1 << 23) != 0                 { self.regs.p = d.addr25; }
            // Conditional branches on S0: JSZ, JSN, JSP, JSM
            0o014 => if self.regs.s[0] == 0                              { self.regs.p = d.addr25; }
            0o015 => if self.regs.s[0] != 0                              { self.regs.p = d.addr25; }
            0o016 => if (self.regs.s[0] as i64) > 0                     { self.regs.p = d.addr25; }
            0o017 => if (self.regs.s[0] as i64) < 0                     { self.regs.p = d.addr25; }

            // VL: transmit (Ak) to VL register
            0o020 => self.regs.write_vl(self.regs.a[k] as u8),
            // VM: transmit (Sj) to VM
            0o033 => self.regs.vm = self.regs.s[j],
            // VM: clear VM
            0o034 => self.regs.vm = 0,

            // --- Address register transmit ---
            // Ai = sign-extended 22-bit constant (addr22)
            0o021 => { self.regs.write_a(i, d.addr22); self.ar_ready_at[i] = 0; }
            // Ai = jk (6-bit constant from same parcel)
            0o022 => { self.regs.write_a(i, d.jk as u32); self.ar_ready_at[i] = 0; }
            // Ai = Sj (lower 24 bits)
            0o023 => { self.regs.write_a(i, self.regs.s[j] as u32); self.ar_ready_at[i] = 0; }
            // Ai = Bjk
            0o024 => { self.regs.write_a(i, self.regs.b[j << 3 | k]); self.ar_ready_at[i] = 0; }
            // Bjk = Ai
            0o025 => self.regs.write_b(j << 3 | k, self.regs.a[i]),

            // --- Address integer arithmetic ---
            // Ai = Aj + Ak  (24-bit, wraps); 2 CP latency
            0o030 => { self.regs.write_a(i, self.regs.a[j].wrapping_add(self.regs.a[k])); self.ar_ready_at[i] = issued_at + 2; }
            // Ai = Aj - Ak; 2 CP latency
            0o031 => { self.regs.write_a(i, self.regs.a[j].wrapping_sub(self.regs.a[k])); self.ar_ready_at[i] = issued_at + 2; }
            // Ai = Aj * Ak  (lower 24 bits of product); 6 CP latency
            0o032 => { self.regs.write_a(i, self.regs.a[j].wrapping_mul(self.regs.a[k])); self.ar_ready_at[i] = issued_at + 6; }

            // --- Scalar transmit ---
            // Si = 22-bit constant (zero-extended)
            0o040 => { self.regs.s[i] = d.addr22 as u64; self.sr_ready_at[i] = 0; }
            // Si = 0
            0o043 => { self.regs.s[i] = 0; self.sr_ready_at[i] = 0; }
            // Si = Ak (zero-extended from 24-bit)
            0o071 if d.j == 0 => { self.regs.s[i] = self.regs.a[k] as u64; self.sr_ready_at[i] = 0; }
            // Si = Ak (sign-extended from 24-bit)
            0o071 if d.j == 1 => { self.regs.s[i] = self.regs.a[k] as i32 as i64 as u64; self.sr_ready_at[i] = 0; }
            // Si = Ak as unnormalized floating point (j=2)
            0o071 if d.j == 2 => { self.regs.s[i] = fp::from_f64(self.regs.a[k] as f64); self.sr_ready_at[i] = 0; }
            // Si = Tjk
            0o074 => { self.regs.s[i] = self.regs.t[j << 3 | k]; self.sr_ready_at[i] = 0; }
            // Tjk = Si
            0o075 => self.regs.t[j << 3 | k] = self.regs.s[i],
            // Si = VM
            0o073 => { self.regs.s[i] = self.regs.vm; self.sr_ready_at[i] = 0; }
            // Si = Vj[Ak]
            0o076 => {
                let elem = self.regs.a[k] as usize & 63;
                self.regs.s[i] = self.regs.v[j][elem];
                self.sr_ready_at[i] = 0;
            }
            // Vi[Ak] = Sj
            0o077 => {
                let elem = self.regs.a[k] as usize & 63;
                self.regs.v[i][elem] = self.regs.s[j];
            }

            // --- Scalar floating point (0o062-0o070); latencies: add=6, mul=7, recip=14 CP ---
            // Si = Sj + Sk (FP add; j=0 with S0=0 normalizes Sk)
            0o062 => { self.regs.s[i] = fp::from_f64(fp::to_f64(self.regs.s[j]) + fp::to_f64(self.regs.s[k])); self.sr_ready_at[i] = issued_at + 6; }
            // Si = Sj - Sk (FP sub; j=0 with S0=0 negates and normalizes Sk)
            0o063 => { self.regs.s[i] = fp::from_f64(fp::to_f64(self.regs.s[j]) - fp::to_f64(self.regs.s[k])); self.sr_ready_at[i] = issued_at + 6; }
            // Si = Sj * Sk (FP multiply, truncated); 7 CP
            0o064 => { self.regs.s[i] = fp::from_f64(fp::to_f64(self.regs.s[j]) * fp::to_f64(self.regs.s[k])); self.sr_ready_at[i] = issued_at + 7; }
            // Si = Sj * Sk (half-precision rounded); 7 CP
            0o065 => { self.regs.s[i] = fp::from_f64(fp::to_f64(self.regs.s[j]) * fp::to_f64(self.regs.s[k])); self.sr_ready_at[i] = issued_at + 7; }
            // Si = Sj * Sk (full-precision rounded); 7 CP
            0o066 => { self.regs.s[i] = fp::from_f64(fp::to_f64(self.regs.s[j]) * fp::to_f64(self.regs.s[k])); self.sr_ready_at[i] = issued_at + 7; }
            // Si = 2 * Sj * Sk; 7 CP
            0o067 => { self.regs.s[i] = fp::from_f64(2.0 * fp::to_f64(self.regs.s[j]) * fp::to_f64(self.regs.s[k])); self.sr_ready_at[i] = issued_at + 7; }
            // Si = reciprocal approximation of Sj; 14 CP
            0o070 => { self.regs.s[i] = fp::from_f64(fp::to_f64(self.regs.s[j]).recip()); self.sr_ready_at[i] = issued_at + 14; }

            // --- Scalar integer arithmetic; 3 CP ---
            // Si = Sj + Sk
            0o060 => { self.regs.s[i] = self.regs.s[j].wrapping_add(self.regs.s[k]); self.sr_ready_at[i] = issued_at + 3; }
            // Si = Sj - Sk
            0o061 => { self.regs.s[i] = self.regs.s[j].wrapping_sub(self.regs.s[k]); self.sr_ready_at[i] = issued_at + 3; }

            // --- Scalar logical; 1 CP ---
            // Si = Sj & Sk
            0o044 => { self.regs.s[i] = self.regs.s[j] & self.regs.s[k]; self.sr_ready_at[i] = issued_at + 1; }
            // Si = Sj & ~Sk
            0o045 => { self.regs.s[i] = self.regs.s[j] & !self.regs.s[k]; self.sr_ready_at[i] = issued_at + 1; }
            // Si = Sj ^ Sk
            0o046 => { self.regs.s[i] = self.regs.s[j] ^ self.regs.s[k]; self.sr_ready_at[i] = issued_at + 1; }
            // Si = ~(Sj ^ Sk)  (logical equivalence / XNOR)
            0o047 => { self.regs.s[i] = !(self.regs.s[j] ^ self.regs.s[k]); self.sr_ready_at[i] = issued_at + 1; }
            // Si = (Si & ~Sk) | (Sj & Sk)  (merge: select Sj where Sk=1, Si where Sk=0)
            0o050 => { self.regs.s[i] = (self.regs.s[i] & !self.regs.s[k]) | (self.regs.s[j] & self.regs.s[k]); self.sr_ready_at[i] = issued_at + 1; }
            // Si = (Si & ~mask) | (Sj & mask) where mask = sign bit of Sj broadcast
            0o051 => {
                let mask = ((self.regs.s[j] as i64) >> 63) as u64;
                self.regs.s[i] = (self.regs.s[i] & !mask) | (self.regs.s[j] & mask);
                self.sr_ready_at[i] = issued_at + 1;
            }

            // --- Scalar shifts; immediate=2 CP, register=3 CP ---
            // S0 = Si << jk
            0o052 => { self.regs.s[0] = self.regs.s[i] << d.jk; self.sr_ready_at[0] = issued_at + 2; }
            // S0 = Si >> (64 - jk)
            0o053 => { self.regs.s[0] = if d.jk == 0 { 0 } else { self.regs.s[i] >> (64 - d.jk) }; self.sr_ready_at[0] = issued_at + 2; }
            // Si = Si << jk
            0o054 => { self.regs.s[i] <<= d.jk; self.sr_ready_at[i] = issued_at + 2; }
            // Si = Si >> (64 - jk)
            0o055 => { self.regs.s[i] = if d.jk == 0 { 0 } else { self.regs.s[i] >> (64 - d.jk) }; self.sr_ready_at[i] = issued_at + 2; }
            // Si = high 64 bits of (Si:Sj) << Ak  (double-length left shift); 3 CP
            0o056 => {
                let count = (self.regs.a[k] & 0x7F) as u32;
                self.regs.s[i] = if count == 0 { self.regs.s[i] }
                    else if count >= 64 { self.regs.s[j] << (count - 64) }
                    else { (self.regs.s[i] << count) | (self.regs.s[j] >> (64 - count)) };
                self.sr_ready_at[i] = issued_at + 3;
            }
            // Si = low 64 bits of (Sj:Si) >> Ak  (double-length right shift); 3 CP
            0o057 => {
                let count = (self.regs.a[k] & 0x7F) as u32;
                self.regs.s[i] = if count == 0 { self.regs.s[i] }
                    else if count >= 64 { self.regs.s[j] >> (count - 64) }
                    else { (self.regs.s[j] << (64 - count)) | (self.regs.s[i] >> count) };
                self.sr_ready_at[i] = issued_at + 3;
            }

            // --- Memory load/store ---
            // The Ah base register is encoded in the low 3 bits of the opcode (the h field).
            // Effective word address = Ah + addr22; result/source register is i.
            //
            // 0o100-0o107: Ai = mem[Ah + addr22]  (lower 24 bits of the 64-bit word)
            // 0o110-0o117: mem[Ah + addr22] = Ai
            // 0o120-0o127: Si = mem[Ah + addr22]
            // 0o130-0o137: mem[Ah + addr22] = Si
            0o100..=0o107 => {
                let base = self.regs.a[h];
                let addr = base.wrapping_add(d.addr22) & ADDR_MASK;
                self.regs.write_a(i, self.mem.read(addr) as u32);
                self.ar_ready_at[i] = issued_at + 7;
            }
            0o110..=0o117 => {
                let base = self.regs.a[h];
                let addr = base.wrapping_add(d.addr22) & ADDR_MASK;
                self.mem.write(addr, self.regs.a[i] as u64);
            }
            0o120..=0o127 => {
                let base = self.regs.a[h];
                let addr = base.wrapping_add(d.addr22) & ADDR_MASK;
                self.regs.s[i] = self.mem.read(addr);
                self.sr_ready_at[i] = issued_at + 7;
            }
            0o130..=0o137 => {
                let base = self.regs.a[h];
                let addr = base.wrapping_add(d.addr22) & ADDR_MASK;
                self.mem.write(addr, self.regs.s[i]);
            }

            // --- Vector floating point multiply (0o160-0o167) ---
            0o160 => { let (vl, sv) = (self.regs.vl as usize, fp::to_f64(self.regs.s[j])); for n in 0..vl { self.regs.v[i][n] = fp::from_f64(sv * fp::to_f64(self.regs.v[k][n])); } }
            0o161 => { let vl = self.regs.vl as usize; for n in 0..vl { self.regs.v[i][n] = fp::from_f64(fp::to_f64(self.regs.v[j][n]) * fp::to_f64(self.regs.v[k][n])); } }
            0o162 => { let (vl, sv) = (self.regs.vl as usize, fp::to_f64(self.regs.s[j])); for n in 0..vl { self.regs.v[i][n] = fp::from_f64(sv * fp::to_f64(self.regs.v[k][n])); } }
            0o163 => { let vl = self.regs.vl as usize; for n in 0..vl { self.regs.v[i][n] = fp::from_f64(fp::to_f64(self.regs.v[j][n]) * fp::to_f64(self.regs.v[k][n])); } }
            0o164 => { let (vl, sv) = (self.regs.vl as usize, fp::to_f64(self.regs.s[j])); for n in 0..vl { self.regs.v[i][n] = fp::from_f64(sv * fp::to_f64(self.regs.v[k][n])); } }
            0o165 => { let vl = self.regs.vl as usize; for n in 0..vl { self.regs.v[i][n] = fp::from_f64(fp::to_f64(self.regs.v[j][n]) * fp::to_f64(self.regs.v[k][n])); } }
            0o166 => { let (vl, sv) = (self.regs.vl as usize, fp::to_f64(self.regs.s[j])); for n in 0..vl { self.regs.v[i][n] = fp::from_f64(2.0 * sv * fp::to_f64(self.regs.v[k][n])); } }
            0o167 => { let vl = self.regs.vl as usize; for n in 0..vl { self.regs.v[i][n] = fp::from_f64(2.0 * fp::to_f64(self.regs.v[j][n]) * fp::to_f64(self.regs.v[k][n])); } }

            // --- Vector floating point add/sub (0o170-0o173) ---
            0o170 => { let (vl, sv) = (self.regs.vl as usize, fp::to_f64(self.regs.s[j])); for n in 0..vl { self.regs.v[i][n] = fp::from_f64(sv + fp::to_f64(self.regs.v[k][n])); } }
            0o171 => { let vl = self.regs.vl as usize; for n in 0..vl { self.regs.v[i][n] = fp::from_f64(fp::to_f64(self.regs.v[j][n]) + fp::to_f64(self.regs.v[k][n])); } }
            0o172 => { let (vl, sv) = (self.regs.vl as usize, fp::to_f64(self.regs.s[j])); for n in 0..vl { self.regs.v[i][n] = fp::from_f64(sv - fp::to_f64(self.regs.v[k][n])); } }
            0o173 => { let vl = self.regs.vl as usize; for n in 0..vl { self.regs.v[i][n] = fp::from_f64(fp::to_f64(self.regs.v[j][n]) - fp::to_f64(self.regs.v[k][n])); } }

            // --- Vector floating point reciprocal (0o174) ---
            0o174 => { let vl = self.regs.vl as usize; for n in 0..vl { self.regs.v[i][n] = fp::from_f64(fp::to_f64(self.regs.v[j][n]).recip()); } }

            // --- Vector logical (0o140-0o147) ---
            // VM bit n = bit (63-n) of the vm u64 (Cray-1 bit 0 = MSB convention).
            0o140 => { let (vl, sv) = (self.regs.vl as usize, self.regs.s[j]); for n in 0..vl { self.regs.v[i][n] = sv & self.regs.v[k][n]; } }
            0o141 => { let vl = self.regs.vl as usize; for n in 0..vl { self.regs.v[i][n] = self.regs.v[j][n] & self.regs.v[k][n]; } }
            0o142 => { let (vl, sv) = (self.regs.vl as usize, self.regs.s[j]); for n in 0..vl { self.regs.v[i][n] = sv | self.regs.v[k][n]; } }
            0o143 => { let vl = self.regs.vl as usize; for n in 0..vl { self.regs.v[i][n] = self.regs.v[j][n] | self.regs.v[k][n]; } }
            0o144 => { let (vl, sv) = (self.regs.vl as usize, self.regs.s[j]); for n in 0..vl { self.regs.v[i][n] = sv ^ self.regs.v[k][n]; } }
            0o145 => { let vl = self.regs.vl as usize; for n in 0..vl { self.regs.v[i][n] = self.regs.v[j][n] ^ self.regs.v[k][n]; } }
            0o146 => {
                let (vl, sv, vm) = (self.regs.vl as usize, self.regs.s[j], self.regs.vm);
                for n in 0..vl {
                    self.regs.v[i][n] = if (vm >> (63 - n)) & 1 != 0 { sv } else { self.regs.v[k][n] };
                }
            }
            0o147 => {
                let (vl, vm) = (self.regs.vl as usize, self.regs.vm);
                for n in 0..vl {
                    let val = if (vm >> (63 - n)) & 1 != 0 { self.regs.v[j][n] } else { self.regs.v[k][n] };
                    self.regs.v[i][n] = val;
                }
            }

            // --- Vector shift (0o150-0o153) ---
            // 0o150/0o151: per-element bit shift left/right by Ak
            // 0o152/0o153: double-shift [Vj|Vj] left/right by Ak = circular element rotation
            0o150 => {
                let (vl, count) = (self.regs.vl as usize, self.regs.a[k] & 0x7F);
                for n in 0..vl { self.regs.v[i][n] = if count >= 64 { 0 } else { self.regs.v[j][n] << count }; }
            }
            0o151 => {
                let (vl, count) = (self.regs.vl as usize, self.regs.a[k] & 0x7F);
                for n in 0..vl { self.regs.v[i][n] = if count >= 64 { 0 } else { self.regs.v[j][n] >> count }; }
            }
            0o152 => {
                let vl = self.regs.vl as usize;
                let count = if vl == 0 { 0 } else { (self.regs.a[k] as usize) % vl };
                for n in 0..vl { self.regs.v[i][n] = self.regs.v[j][(n + count) % vl]; }
            }
            0o153 => {
                let vl = self.regs.vl as usize;
                let count = if vl == 0 { 0 } else { (self.regs.a[k] as usize) % vl };
                for n in 0..vl { self.regs.v[i][n] = self.regs.v[j][(n + vl - count) % vl]; }
            }

            // --- Vector integer add (0o154-0o157) ---
            0o154 => { let (vl, sv) = (self.regs.vl as usize, self.regs.s[j]); for n in 0..vl { self.regs.v[i][n] = sv.wrapping_add(self.regs.v[k][n]); } }
            0o155 => { let vl = self.regs.vl as usize; for n in 0..vl { self.regs.v[i][n] = self.regs.v[j][n].wrapping_add(self.regs.v[k][n]); } }
            0o156 => { let (vl, sv) = (self.regs.vl as usize, self.regs.s[j]); for n in 0..vl { self.regs.v[i][n] = sv.wrapping_sub(self.regs.v[k][n]); } }
            0o157 => { let vl = self.regs.vl as usize; for n in 0..vl { self.regs.v[i][n] = self.regs.v[j][n].wrapping_sub(self.regs.v[k][n]); } }

            // --- Vector mask test (0o175) ---
            // k field encodes condition: 0=zero, 1=nonzero, 2=positive(>0), 3=negative(<0)
            0o175 => {
                let vl = self.regs.vl as usize;
                let mut vm = 0u64;
                for n in 0..vl {
                    let elem = self.regs.v[j][n];
                    let set = match k {
                        0 => elem == 0,
                        1 => elem != 0,
                        2 => elem != 0 && (elem >> 63) == 0,
                        3 => (elem >> 63) != 0,
                        _ => false,
                    };
                    if set { vm |= 1u64 << (63 - n); }
                }
                self.regs.vm = vm;
            }

            // --- Vector memory load/store (0o176-0o177) ---
            // 176ixk: Vi[n] = mem[A0 + n*Ak]; k=0 means stride=1
            // 177xjk: mem[A0 + n*Ak] = Vj[n]; k=0 means stride=1
            0o176 => {
                let vl = self.regs.vl as usize;
                let base = self.regs.a[0];
                let stride = if k == 0 { 1 } else { self.regs.a[k] };
                for n in 0..vl {
                    let addr = base.wrapping_add(stride.wrapping_mul(n as u32)) & ADDR_MASK;
                    self.regs.v[i][n] = self.mem.read(addr);
                }
            }
            0o177 => {
                let vl = self.regs.vl as usize;
                let base = self.regs.a[0];
                let stride = if k == 0 { 1 } else { self.regs.a[k] };
                for n in 0..vl {
                    let addr = base.wrapping_add(stride.wrapping_mul(n as u32)) & ADDR_MASK;
                    self.mem.write(addr, self.regs.v[j][n]);
                }
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

    // Encode the second parcel of a long instruction (a bare 16-bit value).
    fn p1(val: u16) -> [u8; 2] { val.to_be_bytes() }

    // Encode a 32-bit memory instruction: first parcel (opcode, i, addr22 upper 6 bits)
    // followed immediately by the second parcel (addr22 lower 16 bits).
    // For word addresses < 65536 the upper 6 bits (jk) are zero.
    fn mem_instr(opcode: u8, i: u8, word_addr: u32) -> [u8; 4] {
        let jk = ((word_addr >> 16) & 0x3F) as u8;
        let [p0, p1_lo] = parcel(opcode, i, (jk >> 3) & 0x7, jk & 0x7);
        let [p1_hi, p1_lo2] = (word_addr as u16).to_be_bytes();
        [p0, p1_lo, p1_hi, p1_lo2]
    }

    #[test]
    fn mem_load_s_from_word_addr() {
        // Write a known value to word 10, then load it into S1 via 0o120 (load S, Ah=A0=0).
        // Program: S1 = mem[0 + 10]
        let [l0, l1, l2, l3] = mem_instr(0o120, 1, 10); // Si = mem[A0 + 10]
        let [ex0, ex1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[l0, l1, l2, l3, ex0, ex1, 0, 0]);
        cpu.mem.write(10, 0xCAFE_0000_1234_5678);
        cpu.step().unwrap(); // load S1 = mem[10]
        assert_eq!(cpu.regs.s[1], 0xCAFE_0000_1234_5678);
    }

    #[test]
    fn mem_store_s_then_load() {
        // S2 = 0x1234; store to word 20; load back into S3.
        let [s0, s1, s2, s3] = mem_instr(0o130, 2, 20); // mem[20] = S2
        let [l0, l1, l2, l3] = mem_instr(0o120, 3, 20); // S3 = mem[20]
        let [ex0, ex1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[s0, s1, s2, s3, l0, l1, l2, l3, ex0, ex1, 0, 0, 0, 0, 0, 0]);
        cpu.regs.s[2] = 0x1234;
        cpu.step().unwrap(); // store
        cpu.step().unwrap(); // load
        assert_eq!(cpu.regs.s[3], 0x1234);
    }

    #[test]
    fn mem_load_a_lower_24_bits() {
        // mem[5] = 0xFFFF_FFFF_00AB_CDEF; load into A1 -> masked to 24 bits = 0xABCDEF
        let [l0, l1, l2, l3] = mem_instr(0o100, 1, 5); // Ai = mem[A0 + 5] (lower 24 bits)
        let [ex0, ex1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[l0, l1, l2, l3, ex0, ex1, 0, 0]);
        cpu.mem.write(5, 0xFFFF_FFFF_00AB_CDEF);
        cpu.step().unwrap();
        assert_eq!(cpu.regs.a[1], 0x00AB_CDEF);
    }

    #[test]
    fn mem_store_a_then_reload() {
        // A1 = 42; store to word 30; load back into A2.
        let [s0, s1, s2, s3] = mem_instr(0o110, 1, 30); // mem[30] = A1
        let [l0, l1, l2, l3] = mem_instr(0o100, 2, 30); // A2 = mem[30]
        let [ex0, ex1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[s0, s1, s2, s3, l0, l1, l2, l3, ex0, ex1, 0, 0, 0, 0, 0, 0]);
        cpu.regs.a[1] = 42;
        cpu.step().unwrap(); // store
        cpu.step().unwrap(); // load
        assert_eq!(cpu.regs.a[2], 42);
    }

    #[test]
    fn mem_indexed_load() {
        // Use A1 as base (=100), load S3 from mem[A1 + 5] = mem[105].
        // opcode 0o121 = load S with Ah=A1.
        let [l0, l1, l2, l3] = mem_instr(0o121, 3, 5); // S3 = mem[A1 + 5]
        let [ex0, ex1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[l0, l1, l2, l3, ex0, ex1, 0, 0]);
        cpu.regs.a[1] = 100;
        cpu.mem.write(105, 0xDEAD_BEEF);
        cpu.step().unwrap();
        assert_eq!(cpu.regs.s[3], 0xDEAD_BEEF);
    }

    #[test]
    fn branch_unconditional() {
        // J exp (0o006): jump from P=0 over error at P=2 to normal exit at P=4.
        // Layout: [J_p0][J_p1=4][err][exit]
        let [j0, j1] = parcel(0o06, 0, 0, 0); // J (first parcel, addr25 upper 9 bits = 0)
        let [t0, t1] = p1(4);                   // second parcel: target = parcel 4
        let [er0, er1] = parcel(0o00, 0, 0, 0); // error exit at P=2 — must NOT run
        let [pad0, pad1] = [0u8, 0u8];          // unused parcel 3
        let [ex0, ex1] = parcel(0o04, 0, 0, 0); // normal exit at P=4
        let prog = [j0, j1, t0, t1, er0, er1, pad0, pad1, ex0, ex1, 0, 0, 0, 0, 0, 0];
        let mut cpu = cpu_with_program(&prog);
        cpu.step().unwrap();  // J: should set P=4, not 2
        assert_eq!(cpu.regs.p, 4);
        assert_eq!(cpu.step(), Err(Trap::NormalExit));
    }

    #[test]
    fn branch_jaz_taken() {
        // A0 = 0; JAZ jumps to parcel 4 (normal exit), skipping error at parcel 3.
        let [a0, a1] = parcel_jk(0o22, 0, 0); // A0 = 0  (P=0)
        let [j0, j1] = parcel(0o10, 0, 0, 0); // JAZ first parcel  (P=1)
        let [t0, t1] = p1(4);                  // second parcel: target = parcel 4  (P=2)
        let [er, _]  = parcel(0o00, 0, 0, 0);  // error exit  (P=3 — not reached)
        let [ex0, ex1] = parcel(0o04, 0, 0, 0);
        let prog = [a0, a1, j0, j1, t0, t1, er, 0, ex0, ex1, 0, 0, 0, 0, 0, 0];
        let mut cpu = cpu_with_program(&prog);
        cpu.step().unwrap(); // A0 = 0
        cpu.step().unwrap(); // JAZ taken: P should become 4
        assert_eq!(cpu.regs.p, 4);
        assert_eq!(cpu.step(), Err(Trap::NormalExit));
    }

    #[test]
    fn branch_jaz_not_taken() {
        // A0 = 5; JAZ is NOT taken; falls through to normal exit at P=3.
        let [a0, a1] = parcel_jk(0o22, 0, 5); // A0 = 5  (P=0)
        let [j0, j1] = parcel(0o10, 0, 0, 0); // JAZ first parcel  (P=1)
        let [t0, t1] = p1(8);                  // second parcel: target = parcel 8 (never reached)
        let [ex0, ex1] = parcel(0o04, 0, 0, 0); // normal exit at P=3
        let prog = [a0, a1, j0, j1, t0, t1, ex0, ex1, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut cpu = cpu_with_program(&prog);
        cpu.step().unwrap(); // A0 = 5
        cpu.step().unwrap(); // JAZ not taken: P stays at 3
        assert_eq!(cpu.regs.p, 3);
        assert_eq!(cpu.step(), Err(Trap::NormalExit));
    }

    #[test]
    fn fp_add() {
        // S3 = S1 + S2 in floating point
        let [a0, a1] = parcel(0o062, 3, 1, 2);  // S3 = S1 + S2
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, x0, x1]);
        cpu.regs.s[1] = crate::fp::from_f64(1.5);
        cpu.regs.s[2] = crate::fp::from_f64(2.5);
        cpu.step().unwrap();
        assert_eq!(crate::fp::to_f64(cpu.regs.s[3]), 4.0);
    }

    #[test]
    fn fp_mul() {
        // S3 = S1 * S2
        let [a0, a1] = parcel(0o064, 3, 1, 2);
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, x0, x1]);
        cpu.regs.s[1] = crate::fp::from_f64(3.0);
        cpu.regs.s[2] = crate::fp::from_f64(4.0);
        cpu.step().unwrap();
        assert_eq!(crate::fp::to_f64(cpu.regs.s[3]), 12.0);
    }

    #[test]
    fn fp_recip() {
        // S2 = 1/S1
        let [a0, a1] = parcel(0o070, 2, 1, 0);
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, x0, x1]);
        cpu.regs.s[1] = crate::fp::from_f64(4.0);
        cpu.step().unwrap();
        assert_eq!(crate::fp::to_f64(cpu.regs.s[2]), 0.25);
    }

    #[test]
    fn fp_normalize_via_add() {
        // Normalize S1 by adding 0 (S0=0): S2 = S0 + S1 = normalize(S1)
        let [a0, a1] = parcel(0o062, 2, 0, 1);  // S2 = S0 + S1
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, x0, x1]);
        // Build an unnormalized FP word for 1.0 (bit 47 clear)
        let one = crate::fp::from_f64(1.0);
        let exp = (one >> 48) & 0x7FFF;
        let coeff = one & 0x0000_FFFF_FFFF_FFFF;
        let unnorm = ((exp + 4) << 48) | (coeff >> 4); // shift right 4, compensate exponent
        cpu.regs.s[0] = 0; // additive identity
        cpu.regs.s[1] = unnorm;
        cpu.step().unwrap();
        assert_eq!(crate::fp::to_f64(cpu.regs.s[2]), 1.0);
    }

    #[test]
    fn vector_add_vv() {
        // V0 = V1 + V2 element-wise with VL=4
        let [a0, a1] = parcel_jk(0o22, 1, 4);   // A1 = 4
        let [b0, b1] = parcel(0o20, 0, 0, 1);   // VL = A1
        let [c0, c1] = parcel(0o155, 0, 1, 2);  // V0 = V1 + V2
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, b0, b1, c0, c1, x0, x1]);
        for n in 0..4usize { cpu.regs.v[1][n] = (n as u64 + 1) * 10; } // 10,20,30,40
        for n in 0..4usize { cpu.regs.v[2][n] = n as u64 + 1; }         // 1,2,3,4
        cpu.step().unwrap();
        cpu.step().unwrap();
        cpu.step().unwrap();
        assert_eq!(&cpu.regs.v[0][..4], &[11, 22, 33, 44]);
    }

    #[test]
    fn vector_add_sv() {
        // V0 = S1 + V2 (scalar broadcast) with VL=3
        let [a0, a1] = parcel_jk(0o22, 1, 3);   // A1 = 3
        let [b0, b1] = parcel(0o20, 0, 0, 1);   // VL = A1
        let [c0, c1] = parcel(0o154, 0, 1, 2);  // V0 = S1 + V2
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, b0, b1, c0, c1, x0, x1]);
        cpu.regs.s[1] = 100;
        for n in 0..3usize { cpu.regs.v[2][n] = n as u64; } // 0,1,2
        cpu.step().unwrap();
        cpu.step().unwrap();
        cpu.step().unwrap();
        assert_eq!(&cpu.regs.v[0][..3], &[100, 101, 102]);
    }

    #[test]
    fn vector_and_sv() {
        // V0 = S1 & V2 with VL=2
        let [a0, a1] = parcel_jk(0o22, 1, 2);   // A1 = 2
        let [b0, b1] = parcel(0o20, 0, 0, 1);   // VL = A1
        let [c0, c1] = parcel(0o140, 0, 1, 2);  // V0 = S1 & V2
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, b0, b1, c0, c1, x0, x1]);
        cpu.regs.s[1] = 0x0F;
        cpu.regs.v[2][0] = 0xFF;
        cpu.regs.v[2][1] = 0xAA;
        cpu.step().unwrap();
        cpu.step().unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.regs.v[0][0], 0x0F);
        assert_eq!(cpu.regs.v[0][1], 0x0A);
    }

    #[test]
    fn vector_shift_left() {
        // V0 = V1 << A2 with VL=2, A2=3
        let [a0, a1] = parcel_jk(0o22, 1, 2);   // A1 = 2 (VL)
        let [b0, b1] = parcel_jk(0o22, 2, 3);   // A2 = 3 (shift count)
        let [c0, c1] = parcel(0o20, 0, 0, 1);   // VL = A1
        let [d0, d1] = parcel(0o150, 0, 1, 2);  // V0 = V1 << A2
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0,a1,b0,b1,c0,c1,d0,d1,x0,x1, 0,0,0,0,0,0]);
        cpu.regs.v[1][0] = 1;
        cpu.regs.v[1][1] = 2;
        for _ in 0..4 { cpu.step().unwrap(); }
        assert_eq!(cpu.regs.v[0][0], 8);
        assert_eq!(cpu.regs.v[0][1], 16);
    }

    #[test]
    fn vector_mask_test_zero() {
        // vmtest_z on V1=[0,5,0,3] with VL=4 -> VM bits set for elements 0 and 2
        let [a0, a1] = parcel_jk(0o22, 1, 4);   // A1 = 4
        let [b0, b1] = parcel(0o20, 0, 0, 1);   // VL = A1
        let [c0, c1] = parcel(0o175, 0, 1, 0);  // VM = (V1[n]==0)
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, b0, b1, c0, c1, x0, x1]);
        cpu.regs.v[1][0] = 0;
        cpu.regs.v[1][1] = 5;
        cpu.regs.v[1][2] = 0;
        cpu.regs.v[1][3] = 3;
        cpu.step().unwrap();
        cpu.step().unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.regs.vm, (1u64 << 63) | (1u64 << 61));
    }

    #[test]
    fn vector_merge_vv() {
        // V0 = VM ? V1 : V2 with VL=4; VM selects elements 0 and 2 from V1
        let [a0, a1] = parcel_jk(0o22, 1, 4);   // A1 = 4
        let [b0, b1] = parcel(0o20, 0, 0, 1);   // VL = A1
        let [c0, c1] = parcel(0o147, 0, 1, 2);  // V0 = VM ? V1 : V2
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, b0, b1, c0, c1, x0, x1]);
        cpu.regs.vm = (1u64 << 63) | (1u64 << 61); // elements 0 and 2 from V1
        for n in 0..4usize { cpu.regs.v[1][n] = 100 + n as u64; } // 100,101,102,103
        for n in 0..4usize { cpu.regs.v[2][n] = 200 + n as u64; } // 200,201,202,203
        cpu.step().unwrap();
        cpu.step().unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.regs.v[0][0], 100); // VM=1 -> V1
        assert_eq!(cpu.regs.v[0][1], 201); // VM=0 -> V2
        assert_eq!(cpu.regs.v[0][2], 102); // VM=1 -> V1
        assert_eq!(cpu.regs.v[0][3], 203); // VM=0 -> V2
    }

    #[test]
    fn vector_load_store() {
        // vstore V1[0..3] to words 50,51,52; vload into V2; check match
        let [a0, a1] = parcel_jk(0o22, 0, 50);  // A0 = 50 (base address)
        let [b0, b1] = parcel_jk(0o22, 1, 3);   // A1 = 3
        let [c0, c1] = parcel(0o20, 0, 0, 1);   // VL = A1
        let [d0, d1] = parcel(0o177, 0, 1, 0);  // vstore V1 stride=1
        let [e0, e1] = parcel(0o176, 2, 0, 0);  // vload  V2 stride=1
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0,a1,b0,b1,c0,c1,d0,d1,e0,e1,x0,x1, 0,0,0,0]);
        cpu.regs.v[1][0] = 0xAA;
        cpu.regs.v[1][1] = 0xBB;
        cpu.regs.v[1][2] = 0xCC;
        for _ in 0..5 { cpu.step().unwrap(); }
        assert_eq!(cpu.regs.v[2][0], 0xAA);
        assert_eq!(cpu.regs.v[2][1], 0xBB);
        assert_eq!(cpu.regs.v[2][2], 0xCC);
    }

    #[test]
    fn vector_element_insert_extract() {
        // Insert S1=42 into V2[A3=1], then extract V2[A3=1] back to S0
        let [a0, a1] = parcel_jk(0o22, 3, 1);   // A3 = 1 (element index)
        let [b0, b1] = parcel(0o077, 2, 1, 3);  // V2[A3] = S1
        let [c0, c1] = parcel(0o076, 0, 2, 3);  // S0 = V2[A3]
        let [x0, x1] = parcel(0o04, 0, 0, 0);
        let mut cpu = cpu_with_program(&[a0, a1, b0, b1, c0, c1, x0, x1]);
        cpu.regs.s[1] = 42;
        cpu.step().unwrap();
        cpu.step().unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.regs.s[0], 42);
        assert_eq!(cpu.regs.v[2][1], 42);
    }

    #[test]
    fn branch_jan_loop() {
        // Count A0 down from 5 to 0 via JAN back to the decrement.
        // P=0: A0 = 5
        // P=1: A1 = 1
        // P=2: A0 = A0 - A1   (loop body)
        // P=3: JAN first parcel  -> if A0 ≠ 0 jump to P=2
        // P=4: JAN second parcel (target = 2)
        // P=5: normal exit
        let [a0, a1]   = parcel_jk(0o22, 0, 5); // A0 = 5
        let [b0, b1]   = parcel_jk(0o22, 1, 1); // A1 = 1
        let [dec0, dec1] = parcel(0o31, 0, 0, 1); // A0 = A0 - A1
        let [jan0, jan1] = parcel(0o11, 0, 0, 0); // JAN (ijk=0, upper 9 bits of target)
        let [tgt0, tgt1] = p1(2);                  // second parcel: target = parcel 2
        let [ex0, ex1]   = parcel(0o04, 0, 0, 0);
        let prog = [
            a0, a1, b0, b1, dec0, dec1, jan0, jan1,  // word 0 (P=0..3)
            tgt0, tgt1, ex0, ex1, 0, 0, 0, 0,         // word 1 (P=4..7)
        ];
        let mut cpu = cpu_with_program(&prog);
        while cpu.step().is_ok() {}
        assert_eq!(cpu.regs.a[0], 0);
    }
}
