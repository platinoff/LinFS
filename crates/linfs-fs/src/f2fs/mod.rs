use std::sync::Arc;

use linfs_core::block::Block;

pub const F2FS_MAGIC: u32 = 0xF2F52010;

#[derive(Debug, Clone)]
pub struct Superblock {
    pub magic: u32,
    pub major_ver: u16,
}

pub struct F2fsFs {
    #[allow(dead_code)]
    block: Arc<dyn Block>,
    sb: Superblock,
}

impl F2fsFs {
    pub fn open(block: Arc<dyn Block>) -> linfs_core::Result<Self> {
        let mut buf = [0u8; 512];
        block
            .read_at(0, &mut buf)
            .map_err(|e| linfs_core::Error::Corruption(format!("read f2fs sb: {e}")))?;
        let magic = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        if magic != F2FS_MAGIC {
            return Err(linfs_core::Error::Corruption(format!(
                "f2fs bad magic 0x{magic:08x}"
            )));
        }
        let major_ver = u16::from_le_bytes([buf[4], buf[5]]);
        Ok(Self {
            block,
            sb: Superblock { magic, major_ver },
        })
    }
    pub fn magic(&self) -> u32 {
        self.sb.magic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linfs_core::block::Block;

    struct MemBlock(Vec<u8>);
    impl Block for MemBlock {
        fn read_at(&self, off: u64, buf: &mut [u8]) -> std::io::Result<()> {
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

    fn make_f2fs_sb() -> Vec<u8> {
        let mut data = vec![0u8; 8 * 1024 * 1024];
        data[0..4].copy_from_slice(&F2FS_MAGIC.to_le_bytes());
        data
    }

    #[test]
    fn f2fs_open_valid() {
        let data = make_f2fs_sb();
        let fs = F2fsFs::open(Arc::new(MemBlock(data))).unwrap();
        assert_eq!(fs.magic(), F2FS_MAGIC);
    }

    #[test]
    fn f2fs_rejects_bad_magic() {
        let data = vec![0u8; 8 * 1024 * 1024];
        assert!(F2fsFs::open(Arc::new(MemBlock(data))).is_err());
    }
}
