/// Buddy bitmap allocator for blocks/inodes (bg_block_bitmap / bg_inode_bitmap).
/// For MVP, we operate on in-memory bitmap blocks; real fs writes through journal Tx.
pub struct Bitmap {
    pub bits: Vec<u8>,
}

impl Bitmap {
    pub fn from_block(block_data: &[u8]) -> Self {
        Self {
            bits: block_data.to_vec(),
        }
    }

    /// Find first zero bit, set to 1, return bit index (0-based). None if full.
    pub fn alloc(&mut self) -> Option<u32> {
        for (byte_idx, byte) in self.bits.iter_mut().enumerate() {
            if *byte != 0xFF {
                for bit in 0..8 {
                    if (*byte >> bit) & 1 == 0 {
                        *byte |= 1 << bit;
                        return Some((byte_idx as u32) * 8 + bit as u32);
                    }
                }
            }
        }
        None
    }

    pub fn free(&mut self, idx: u32) {
        let byte_idx = (idx / 8) as usize;
        let bit = (idx % 8) as u8;
        if byte_idx < self.bits.len() {
            self.bits[byte_idx] &= !(1 << bit);
        }
    }

    pub fn is_allocated(&self, idx: u32) -> bool {
        let byte_idx = (idx / 8) as usize;
        let bit = (idx % 8) as u8;
        if byte_idx >= self.bits.len() {
            return false;
        }
        (self.bits[byte_idx] >> bit) & 1 == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmap_alloc_first() {
        let mut bm = Bitmap::from_block(&[0x00, 0xFF, 0x00]);
        assert_eq!(bm.alloc(), Some(0));
        assert_eq!(bm.alloc(), Some(1));
        assert!(bm.is_allocated(0));
        assert!(bm.is_allocated(1));
    }

    #[test]
    fn bitmap_alloc_skips_full_bytes() {
        let mut bm = Bitmap::from_block(&[0xFF, 0x00]);
        assert_eq!(bm.alloc(), Some(8));
        assert_eq!(bm.alloc(), Some(9));
    }

    #[test]
    fn bitmap_free() {
        let mut bm = Bitmap::from_block(&[0xFF]);
        bm.free(3);
        assert!(!bm.is_allocated(3));
        assert_eq!(bm.alloc(), Some(3));
    }

    #[test]
    fn bitmap_full_returns_none() {
        let mut bm = Bitmap::from_block(&[0xFF, 0xFF]);
        assert_eq!(bm.alloc(), None);
    }
}
