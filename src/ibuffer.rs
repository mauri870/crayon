use crate::memory::Memory;

// Each buffer holds 64 parcels = 16 64-bit memory words.
const BUFFER_PARCELS: usize = 64;
const BUFFER_WORDS: usize = BUFFER_PARCELS / 4;

struct Buffer {
    parcels: [u16; BUFFER_PARCELS],
    // Word address of parcel 0 in this buffer.
    base: u32,
    valid: bool,
}

impl Buffer {
    fn new() -> Self {
        Self {
            parcels: [0; BUFFER_PARCELS],
            base: 0,
            valid: false,
        }
    }

    fn contains(&self, word_addr: u32) -> bool {
        self.valid && word_addr >= self.base && word_addr < self.base + BUFFER_WORDS as u32
    }

    fn fetch(&self, word_addr: u32, parcel_idx: usize) -> u16 {
        let offset = ((word_addr - self.base) as usize) * 4 + parcel_idx;
        self.parcels[offset]
    }

    fn fill(&mut self, base: u32, mem: &Memory) {
        self.base = base;
        self.valid = true;
        for w in 0..BUFFER_WORDS {
            let word = mem.read(base + w as u32);
            for p in 0..4 {
                self.parcels[w * 4 + p] = ((word >> (48 - p * 16)) & 0xFFFF) as u16;
            }
        }
    }
}

// Four instruction buffers, filled in rotation (LRU approximation).
pub struct InstructionBuffers {
    buffers: [Buffer; 4],
    // Index of the buffer filled least recently (next to be evicted).
    next_fill: usize,
}

impl InstructionBuffers {
    pub fn new() -> Self {
        Self {
            buffers: [Buffer::new(), Buffer::new(), Buffer::new(), Buffer::new()],
            next_fill: 0,
        }
    }

    // Return the parcel at parcel-counter value p, filling from memory if needed.
    // p bits 23:2 = word address, bits 1:0 = parcel index within the word.
    pub fn fetch(&mut self, p: u32, mem: &Memory) -> u16 {
        let word_addr = p >> 2;
        let parcel_idx = (p & 0x3) as usize;

        for buf in &self.buffers {
            if buf.contains(word_addr) {
                return buf.fetch(word_addr, parcel_idx);
            }
        }

        // Miss: fill the next buffer in rotation starting at the current word.
        let slot = self.next_fill;
        self.buffers[slot].fill(word_addr, mem);
        self.next_fill = (slot + 1) % 4;
        self.buffers[slot].fetch(word_addr, parcel_idx)
    }
}

impl Default for InstructionBuffers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Memory;

    fn mem_with_parcel(parcel: u16) -> Memory {
        let mut mem = Memory::new();
        // Place parcel in the first slot of word 0.
        mem.write(0, (parcel as u64) << 48);
        mem
    }

    #[test]
    fn fetch_from_word_0_parcel_0() {
        let mem = mem_with_parcel(0x6053);
        let mut ibufs = InstructionBuffers::new();
        assert_eq!(ibufs.fetch(0, &mem), 0x6053);
    }

    #[test]
    fn second_fetch_hits_buffer() {
        let mem = mem_with_parcel(0x6053);
        let mut ibufs = InstructionBuffers::new();
        ibufs.fetch(0, &mem); // fills buffer 0
        // next_fill advanced to 1 on miss; a second fetch must hit buffer 0
        let p = ibufs.fetch(0, &mem);
        assert_eq!(p, 0x6053);
        assert_eq!(ibufs.next_fill, 1); // no second fill occurred
    }

    #[test]
    fn fetch_parcel_index_1() {
        let mut mem = Memory::new();
        // Pack two distinct parcels into word 0.
        mem.write(0, (0x1234u64 << 48) | (0x5678u64 << 32));
        let mut ibufs = InstructionBuffers::new();
        assert_eq!(ibufs.fetch(0, &mem), 0x1234); // parcel 0
        assert_eq!(ibufs.fetch(1, &mem), 0x5678); // parcel 1
    }
}
