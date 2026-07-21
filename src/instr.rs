// A parcel is a 16-bit instruction slot. Four fit in one 64-bit memory word.
//
// Bit layout:
//
//   15    12 11   9 8    6 5   3 2   0
//   [  g:4  ][ h:3 ][ i:3 ][ j:3 ][ k:3 ]
//
// The 7-bit opcode is g:h. i, j, k are 3-bit register indices.
// For shift/mask instructions jk is a 6-bit count.
//
// Two-parcel (32-bit) instructions extend the address using the next parcel:
//   22-bit word address:    jk(6) from parcel 0 | parcel 1(16)
//   25-bit parcel address: ijk(9) from parcel 0 | parcel 1(16)

#[derive(Debug, Clone, Copy)]
pub struct Decoded {
    pub opcode: u8,  // 7-bit: g(4):h(3)
    pub i: u8,       // result register index
    pub j: u8,       // first operand register index
    pub k: u8,       // second operand register index
    pub jk: u8,      // 6-bit shift/mask count
    pub addr22: u32, // 22-bit word address (jk | parcel 1)
    pub addr25: u32, // 25-bit parcel address (ijk | parcel 1)
    pub long: bool,  // true when two parcels were consumed
}

// Decode one instruction. p1 is only used when the opcode is two-parcel.
pub fn decode(p0: u16, p1: u16) -> Decoded {
    let g  = ((p0 >> 12) & 0xF) as u8;
    let h  = ((p0 >>  9) & 0x7) as u8;
    let i  = ((p0 >>  6) & 0x7) as u8;
    let j  = ((p0 >>  3) & 0x7) as u8;
    let k  = ( p0        & 0x7) as u8;
    let jk = ( p0        & 0x3F) as u8;

    let opcode = (g << 3) | h;
    let long   = is_long(opcode, i);

    // The two extended addresses are always computed so callers can use them
    // without an extra branch, even for single-parcel instructions.
    let addr22 = ((jk as u32) << 16) | (p1 as u32);
    let addr25 = (((p0 & 0x1FF) as u32) << 16) | (p1 as u32);

    Decoded { opcode, i, j, k, jk, addr22, addr25, long }
}

// Returns true for opcodes that consume a second parcel.
fn is_long(opcode: u8, i: u8) -> bool {
    match opcode {
        // Unconditional jump / return with explicit parcel address
        0o06 | 0o07 => true,
        // Jump to Bjk is 16-bit (i=0); jump to exp is 32-bit (i≠0)
        0o05 => i != 0,
        // Conditional branches (JAZ, JAN, JAP, JAM, JSZ, JSN, JSP, JSM)
        0o10..=0o17 => true,
        // Transmit 22-bit constant to A register (zero-extended or 1's complement)
        0o20 | 0o21 => true,
        // Transmit 22-bit constant to S register (zero-extended or sign-extended)
        0o40 | 0o41 => true,
        // Memory read/store with explicit word address
        0o100..=0o137 => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_16bit_scalar_add() {
        // 060ijk: Si = Sj + Sk  (opcode 0o60 = 48, 16-bit)
        // g=6(0110), h=0(000), i=1, j=2, k=3 -> 0110_000_001_010_011 = 0x6053
        let d = decode(0x6053, 0x0000);
        assert_eq!(d.opcode, 0o60);
        assert_eq!(d.i, 1);
        assert_eq!(d.j, 2);
        assert_eq!(d.k, 3);
        assert!(!d.long);
    }

    #[test]
    fn decode_32bit_memory_load() {
        // 120ijk: Read from (exp + Ah) to Si — 32-bit
        // opcode 0o120 = 80, g=0b1010=10, h=0, i=1, j=0, k=0
        // addr22 = jk(0) | p1(0x0004) = 4
        let d = decode(0xA040, 0x0004);
        assert_eq!(d.opcode, 0o120);
        assert_eq!(d.i, 1);
        assert_eq!(d.addr22, 4);
        assert!(d.long);
    }

    #[test]
    fn decode_32bit_branch() {
        // 010ijk JAZ: branch if A0=0, 32-bit parcel address
        // opcode 0o10 = 8: g=1 (0001b -> bits 15:12), h=0 (000b -> bits 11:9)
        // parcel = 0001_000_000_000_000b = 0x1000; addr25 = ijk(0) | p1(0x000A) = 10
        let d = decode(0x1000, 0x000A);
        assert_eq!(d.opcode, 0o10);
        assert!(d.long);
        assert_eq!(d.addr25, 10);
    }

    #[test]
    fn short_jump_to_register() {
        // 005xjk J Bjk: i=0 -> 16-bit
        // opcode 0o05 = 5: g=0, h=5 (101b -> bits 11:9) -> 0x0A00; i=0
        let d = decode(0x0A00, 0x0000);
        assert_eq!(d.opcode, 0o05);
        assert!(!d.long);
    }
}
