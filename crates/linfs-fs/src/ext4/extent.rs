pub const EXTENT_MAGIC: u16 = 0xF30A;

#[derive(Debug, Clone)]
pub struct ExtentHeader {
    pub magic: u16,
    pub entries: u16,
    pub max: u16,
    pub depth: u16,
    pub generation: u32,
}

#[derive(Debug, Clone)]
pub struct Extent {
    pub block: u32, // logical block
    pub len: u16,   // number of blocks (if >32768 unwritten)
    pub start_hi: u16,
    pub start_lo: u32,
}

impl Extent {
    pub fn physical(&self) -> u64 {
        (self.start_lo as u64) | ((self.start_hi as u64) << 32)
    }
    pub fn is_unwritten(&self) -> bool {
        self.len > 0x8000
    }
    pub fn len_blocks(&self) -> u32 {
        (self.len & 0x7FFF) as u32
    }
}

#[derive(Debug, Clone)]
pub struct ExtentIdx {
    pub block: u32,
    pub leaf_lo: u32,
    pub leaf_hi: u16,
    pub unused: u16,
}

pub fn parse_extent_header(buf: &[u8]) -> linfs_core::Result<ExtentHeader> {
    if buf.len() < 12 {
        return Err(linfs_core::Error::Corruption(
            "extent header too short".into(),
        ));
    }
    let magic = u16::from_le_bytes([buf[0], buf[1]]);
    if magic != EXTENT_MAGIC {
        return Err(linfs_core::Error::Corruption(format!(
            "extent bad magic 0x{magic:04x}"
        )));
    }
    Ok(ExtentHeader {
        magic,
        entries: u16::from_le_bytes([buf[2], buf[3]]),
        max: u16::from_le_bytes([buf[4], buf[5]]),
        depth: u16::from_le_bytes([buf[6], buf[7]]),
        generation: u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
    })
}

pub fn parse_extent(buf: &[u8]) -> Extent {
    Extent {
        block: u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
        len: u16::from_le_bytes([buf[4], buf[5]]),
        start_hi: u16::from_le_bytes([buf[6], buf[7]]),
        start_lo: u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
    }
}

pub fn parse_extent_idx(buf: &[u8]) -> ExtentIdx {
    ExtentIdx {
        block: u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
        leaf_lo: u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
        leaf_hi: u16::from_le_bytes([buf[8], buf[9]]),
        unused: u16::from_le_bytes([buf[10], buf[11]]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extent_header_parse() {
        let mut buf = [0u8; 12];
        buf[0..2].copy_from_slice(&EXTENT_MAGIC.to_le_bytes());
        buf[2..4].copy_from_slice(&1u16.to_le_bytes());
        buf[4..6].copy_from_slice(&4u16.to_le_bytes());
        buf[6..8].copy_from_slice(&0u16.to_le_bytes());
        let h = parse_extent_header(&buf).unwrap();
        assert_eq!(h.magic, EXTENT_MAGIC);
        assert_eq!(h.entries, 1);
        assert_eq!(h.depth, 0);
    }

    #[test]
    fn extent_physical() {
        let e = Extent {
            block: 0,
            len: 4,
            start_hi: 0,
            start_lo: 100,
        };
        assert_eq!(e.physical(), 100);
        assert!(!e.is_unwritten());
        assert_eq!(e.len_blocks(), 4);
    }

    #[test]
    fn extent_header_bad_magic() {
        let buf = [0u8; 12];
        assert!(parse_extent_header(&buf).is_err());
    }
}
