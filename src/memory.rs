pub const MEMORY_WORDS: usize = 1 << 20; // 1M 64-bit words
pub const BANK_COUNT: usize = 16;

pub struct Memory {
    words: Vec<u64>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            words: vec![0u64; MEMORY_WORDS],
        }
    }

    pub fn read(&self, addr: u32) -> u64 {
        self.words[(addr as usize) & (MEMORY_WORDS - 1)]
    }

    pub fn write(&mut self, addr: u32, value: u64) {
        self.words[(addr as usize) & (MEMORY_WORDS - 1)] = value;
    }

    // Load a flat binary image into memory starting at word address 0.
    // Bytes are big-endian: the first byte of each 8-byte group lands in bits 63:56.
    pub fn load_program(&mut self, data: &[u8]) {
        for (i, chunk) in data.chunks(8).enumerate() {
            let mut word = 0u64;
            for (b, &byte) in chunk.iter().enumerate() {
                word |= (byte as u64) << (56 - b * 8);
            }
            self.words[i] = word;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write_roundtrip() {
        let mut mem = Memory::new();
        mem.write(0x100, 0xDEADBEEF_CAFEBABE);
        assert_eq!(mem.read(0x100), 0xDEADBEEF_CAFEBABE);
    }

    #[test]
    fn load_program_big_endian() {
        let mut mem = Memory::new();
        mem.load_program(&[0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF]);
        assert_eq!(mem.read(0), 0x0123456789ABCDEF);
    }

    #[test]
    fn address_wraps_at_1m() {
        let mut mem = Memory::new();
        mem.write(0, 0xAB);
        assert_eq!(mem.read(MEMORY_WORDS as u32), 0xAB);
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}
