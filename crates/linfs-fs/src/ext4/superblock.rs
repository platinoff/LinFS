#[derive(Debug, Clone)]
pub struct Superblock {
    pub block_size: u32,
    pub blocks_count: u64,
    pub blocks_per_group: u32,
    pub inodes_per_group: u32,
    pub inode_size: u16,
    pub first_ino: u32,
    pub rev_level: u32,
    pub feature_incompat: u32,
    pub feature_compat: u32,
    pub feature_ro_compat: u32,
}

impl Superblock {
    /// Read 1024-byte superblock at byte offset 1024 and validate `s_magic == 0xEF53`.
    pub fn read(block: &dyn linfs_core::block::Block) -> linfs_core::Result<Self> {
        let mut buf = [0u8; 1024];
        block
            .read_at(1024, &mut buf)
            .map_err(|e| linfs_core::Error::Corruption(format!("read superblock: {e}")))?;

        let magic = u16::from_le_bytes([buf[56], buf[57]]);
        if magic != 0xEF53 {
            return Err(linfs_core::Error::Corruption(format!(
                "ext4 bad magic 0x{magic:04x} != 0xEF53"
            )));
        }

        let s_log_block_size = u32::from_le_bytes([buf[24], buf[25], buf[26], buf[27]]);
        if s_log_block_size > 6 {
            return Err(linfs_core::Error::Corruption(format!(
                "s_log_block_size out of range: {s_log_block_size}"
            )));
        }
        let block_size = 1024u32
            .checked_shl(s_log_block_size)
            .ok_or_else(|| linfs_core::Error::Corruption("block_size overflow".into()))?;

        let blocks_count_lo = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]) as u64;
        let s_blocks_count_hi = if buf.len() >= 0x150 + 4 {
            u32::from_le_bytes([buf[0x14C], buf[0x14D], buf[0x14E], buf[0x14F]]) as u64
        } else {
            0
        };
        let blocks_count = blocks_count_lo | (s_blocks_count_hi << 32);

        let blocks_per_group = u32::from_le_bytes([buf[32], buf[33], buf[34], buf[35]]);
        let inodes_per_group = u32::from_le_bytes([buf[40], buf[41], buf[42], buf[43]]);
        let inode_size = u16::from_le_bytes([buf[88], buf[89]]);
        let first_ino = u32::from_le_bytes([buf[84], buf[85], buf[86], buf[87]]);
        let rev_level = u32::from_le_bytes([buf[76], buf[77], buf[78], buf[79]]);
        let feature_compat = u32::from_le_bytes([buf[92], buf[93], buf[94], buf[95]]);
        let feature_incompat = u32::from_le_bytes([buf[96], buf[97], buf[98], buf[99]]);
        let feature_ro_compat = u32::from_le_bytes([buf[100], buf[101], buf[102], buf[103]]);

        if inode_size != 0 && !(128..=1024).contains(&inode_size) {
            return Err(linfs_core::Error::Corruption(format!(
                "bad s_inode_size {inode_size}"
            )));
        }

        Ok(Self {
            block_size,
            blocks_count,
            blocks_per_group,
            inodes_per_group,
            inode_size: if inode_size == 0 { 128 } else { inode_size },
            first_ino: if first_ino == 0 { 11 } else { first_ino },
            rev_level,
            feature_incompat,
            feature_compat,
            feature_ro_compat,
        })
    }
}
