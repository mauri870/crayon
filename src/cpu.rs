use std::fmt;

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
