/// Raw block device — Windows PhysicalDrive or image file.
/// All offsets are in bytes; reads/writes must be sector-aligned by caller if needed.
pub trait Block: Send + Sync {
    fn read_at(&self, off: u64, buf: &mut [u8]) -> std::io::Result<()>;
    fn write_at(&self, off: u64, buf: &[u8]) -> std::io::Result<()>;
    fn len(&self) -> u64;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn sector_size(&self) -> u32 {
        512
    }
}

/// Slice of a parent Block (a partition or LV).
pub struct SliceBlock {
    pub parent: std::sync::Arc<dyn Block>,
    pub offset: u64,
    pub length: u64,
    pub sector: u32,
}

impl Block for SliceBlock {
    fn read_at(&self, off: u64, buf: &mut [u8]) -> std::io::Result<()> {
        if off + buf.len() as u64 > self.length {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "slice oob",
            ));
        }
        self.parent.read_at(self.offset + off, buf)
    }
    fn write_at(&self, off: u64, buf: &[u8]) -> std::io::Result<()> {
        if off + buf.len() as u64 > self.length {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "slice oob",
            ));
        }
        self.parent.write_at(self.offset + off, buf)
    }
    fn len(&self) -> u64 {
        self.length
    }
    fn sector_size(&self) -> u32 {
        self.sector
    }
}
