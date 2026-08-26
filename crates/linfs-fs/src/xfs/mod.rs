use std::sync::Arc;

use linfs_core::block::Block;

#[derive(Debug, Clone)]
pub struct Superblock {
    pub magic: [u8; 4],
    pub block_size: u32,
    pub dblocks: u64,
    pub rblocks: u64,
    pub ag_blocks: u32,
}

pub struct XfsFs {
    #[allow(dead_code)]
    block: Arc<dyn Block>,
    sb: Superblock,
}

impl XfsFs {
    pub fn open(block: Arc<dyn Block>) -> linfs_core::Result<Self> {
        let mut buf = [0u8; 512];
        block
            .read_at(0, &mut buf)
            .map_err(|e| linfs_core::Error::Corruption(format!("read xfs sb: {e}")))?;
        if &buf[0..4] != b"XFSB" {
            return Err(linfs_core::Error::Corruption(format!(
                "xfs bad magic {:02x?}",
                &buf[0..4]
            )));
        }
        let block_size = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
        if !matches!(block_size, 512 | 1024 | 2048 | 4096 | 8192 | 16384) {
            return Err(linfs_core::Error::Corruption(format!(
                "xfs bad blocksize {block_size}"
            )));
        }
        let dblocks = u64::from_be_bytes(buf[8..16].try_into().unwrap());
        let rblocks = u64::from_be_bytes(buf[16..24].try_into().unwrap());
        let ag_blocks = u32::from_be_bytes(buf[84..88].try_into().unwrap_or([0, 0, 0, 0]));
        Ok(Self {
            block,
            sb: Superblock {
                magic: [buf[0], buf[1], buf[2], buf[3]],
                block_size,
                dblocks,
                rblocks,
                ag_blocks,
            },
        })
    }

    pub fn block_size(&self) -> u32 {
        self.sb.block_size
    }

    pub fn superblock(&self) -> &Superblock {
        &self.sb
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

    fn make_xfs_sb(bs: u32) -> Vec<u8> {
        let mut data = vec![0u8; 8 * 1024 * 1024];
        data[0..4].copy_from_slice(b"XFSB");
        data[4..8].copy_from_slice(&bs.to_be_bytes());
        let blocks = (data.len() as u64) / bs as u64;
        data[8..16].copy_from_slice(&blocks.to_be_bytes());
        data
    }

    #[test]
    fn xfs_open_valid() {
        let data = make_xfs_sb(4096);
        let fs = XfsFs::open(Arc::new(MemBlock(data))).unwrap();
        assert_eq!(fs.block_size(), 4096);
    }

    #[test]
    fn xfs_rejects_bad_magic() {
        let mut data = make_xfs_sb(4096);
        data[0] = 0;
        let res = XfsFs::open(Arc::new(MemBlock(data)));
        assert!(res.is_err());
    }
}
