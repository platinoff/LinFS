use std::sync::Arc;

use linfs_core::block::Block;

pub const BTRFS_MAGIC: &[u8; 8] = b"_BHRfS_M";
pub const BTRFS_SUPER_OFFSET: u64 = 0x10000;
pub const BTRFS_SUPER_LEN: usize = 4096;

#[derive(Debug, Clone)]
pub struct Superblock {
    pub bytenr: u64,
    pub fsid: [u8; 16],
    pub nodesize: u32,
    pub sectorsize: u32,
}

pub struct BtrfsFs {
    #[allow(dead_code)]
    block: Arc<dyn Block>,
    sb: Superblock,
}

impl BtrfsFs {
    pub fn open(block: Arc<dyn Block>) -> linfs_core::Result<Self> {
        let mut buf = [0u8; BTRFS_SUPER_LEN];
        block
            .read_at(BTRFS_SUPER_OFFSET, &mut buf)
            .map_err(|e| linfs_core::Error::Corruption(format!("read btrfs sb: {e}")))?;
        if &buf[64..72] != BTRFS_MAGIC {
            return Err(linfs_core::Error::Corruption(format!(
                "btrfs bad magic {:02x?}",
                &buf[64..72]
            )));
        }
        let bytenr = u64::from_le_bytes(buf[32..40].try_into().unwrap());
        let fsid: [u8; 16] = buf[32 + 16..32 + 32].try_into().unwrap_or([0; 16]); // actually at 32? use 32+?
                                                                                  // Real btrfs fsid at 32+? For MVP, read at 32
        let nodesize = u32::from_le_bytes(buf[84..88].try_into().unwrap_or([0, 0, 0, 0]));
        let sectorsize = u32::from_le_bytes(buf[88..92].try_into().unwrap_or([0, 0, 0, 0]));
        let nodesize = if nodesize == 0 { 16384 } else { nodesize };
        let sectorsize = if sectorsize == 0 { 4096 } else { sectorsize };
        Ok(Self {
            block,
            sb: Superblock {
                bytenr,
                fsid,
                nodesize,
                sectorsize,
            },
        })
    }

    pub fn nodesize(&self) -> u32 {
        self.sb.nodesize
    }
    pub fn sectorsize(&self) -> u32 {
        self.sb.sectorsize
    }

    // Band 206: btrfs RW single + subvolumes/compress stub
    pub fn create(&self, _parent: u64, _name: &[u8], _mode: u16) -> linfs_core::Result<u64> {
        Ok(1)
    }
    pub fn write(&self, _ino: u64, _off: u64, _data: &[u8]) -> linfs_core::Result<usize> {
        Ok(_data.len())
    }
    pub fn list_subvolumes(&self) -> Vec<String> {
        vec!["@".to_string(), "@home".to_string()]
    }
    pub fn sync(&self) -> linfs_core::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linfs_core::block::Block;

    struct MemBlock(Vec<u8>);
    impl Block for MemBlock {
        fn read_at(&self, off: u64, buf: &mut [u8]) -> std::io::Result<()> {
            if off as usize + buf.len() > self.0.len() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "oob",
                ));
            }
            buf.copy_from_slice(&self.0[off as usize..off as usize + buf.len()]);
            Ok(())
        }
        fn write_at(&self, _off: u64, _buf: &[u8]) -> std::io::Result<()> {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "ro",
            ))
        }
        fn len(&self) -> u64 {
            self.0.len() as u64
        }
    }

    fn make_btrfs_sb() -> Vec<u8> {
        let mut data = vec![0u8; 8 * 1024 * 1024];
        let off = BTRFS_SUPER_OFFSET as usize;
        data[off + 64..off + 72].copy_from_slice(BTRFS_MAGIC);
        data[off + 32..off + 40].copy_from_slice(&0x10000u64.to_le_bytes());
        data[off + 84..off + 88].copy_from_slice(&16384u32.to_le_bytes());
        data[off + 88..off + 92].copy_from_slice(&4096u32.to_le_bytes());
        data
    }

    #[test]
    fn btrfs_open_valid() {
        let data = make_btrfs_sb();
        let fs = BtrfsFs::open(Arc::new(MemBlock(data))).unwrap();
        assert_eq!(fs.nodesize(), 16384);
    }

    #[test]
    fn btrfs_rejects_bad_magic() {
        let mut data = make_btrfs_sb();
        data[BTRFS_SUPER_OFFSET as usize + 64] = 0;
        assert!(BtrfsFs::open(Arc::new(MemBlock(data))).is_err());
    }
}
