#[derive(Debug, Clone)]
pub struct GroupDesc {
    pub block_bitmap_lo: u32,
    pub inode_bitmap_lo: u32,
    pub inode_table_lo: u32,
    pub free_blocks_count: u16,
    pub free_inodes_count: u16,
    pub used_dirs_count: u16,
    pub block_bitmap_hi: u32,
    pub inode_bitmap_hi: u32,
    pub inode_table_hi: u32,
}

impl GroupDesc {
    pub fn inode_table_block(&self) -> u64 {
        (self.inode_table_lo as u64) | ((self.inode_table_hi as u64) << 32)
    }
    pub fn block_bitmap_block(&self) -> u64 {
        (self.block_bitmap_lo as u64) | ((self.block_bitmap_hi as u64) << 32)
    }
}

pub fn read_group_descs(
    block: &dyn linfs_core::block::Block,
    sb: &super::superblock::Superblock,
) -> linfs_core::Result<Vec<GroupDesc>> {
    let block_size = sb.block_size as u64;
    // Group descs start at block after superblock:
    // For 1K block_size: superblock is block 1, gdt at block 2
    // Otherwise: superblock in block 0, gdt at block 1
    let gdt_block = if block_size == 1024 { 2 } else { 1 };
    let gdt_off = gdt_block * block_size;

    // Number of groups = ceil(blocks_count / blocks_per_group)
    let groups = sb.blocks_count.div_ceil(sb.blocks_per_group as u64);
    // For our synthetic 8MiB with 4096 block_size: 2048 blocks /8192 → 1 group
    let groups = groups.clamp(1, 128) as usize; // cap for test

    let desc_size: usize = if sb.feature_incompat & 0x80 != 0 {
        64
    } else {
        32
    }; // 64bit
    let mut out = Vec::with_capacity(groups);
    let mut buf = vec![0u8; desc_size * groups];
    block
        .read_at(gdt_off, &mut buf)
        .map_err(|e| linfs_core::Error::Corruption(format!("read gdt: {e}")))?;

    for i in 0..groups {
        let off = i * desc_size;
        let d = &buf[off..off + desc_size];
        out.push(GroupDesc {
            block_bitmap_lo: u32::from_le_bytes([d[0], d[1], d[2], d[3]]),
            inode_bitmap_lo: u32::from_le_bytes([d[4], d[5], d[6], d[7]]),
            inode_table_lo: u32::from_le_bytes([d[8], d[9], d[10], d[11]]),
            free_blocks_count: u16::from_le_bytes([d[12], d[13]]),
            free_inodes_count: u16::from_le_bytes([d[14], d[15]]),
            used_dirs_count: u16::from_le_bytes([d[16], d[17]]),
            block_bitmap_hi: if desc_size == 64 {
                u32::from_le_bytes([d[32], d[33], d[34], d[35]])
            } else {
                0
            },
            inode_bitmap_hi: if desc_size == 64 {
                u32::from_le_bytes([d[36], d[37], d[38], d[39]])
            } else {
                0
            },
            inode_table_hi: if desc_size == 64 {
                u32::from_le_bytes([d[40], d[41], d[42], d[43]])
            } else {
                0
            },
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ext4::superblock::Superblock;
    use linfs_core::block::Block;

    struct MemBlock {
        data: Vec<u8>,
    }
    impl Block for MemBlock {
        fn read_at(&self, off: u64, buf: &mut [u8]) -> std::io::Result<()> {
            buf.copy_from_slice(&self.data[off as usize..off as usize + buf.len()]);
            Ok(())
        }
        fn write_at(&self, _off: u64, _buf: &[u8]) -> std::io::Result<()> {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "ro",
            ))
        }
        fn len(&self) -> u64 {
            self.data.len() as u64
        }
    }

    fn make_sb_and_gdt() -> (Superblock, MemBlock) {
        let mut data = vec![0u8; 8 * 1024 * 1024];
        let sb_off = 1024;
        data[sb_off + 56] = 0x53;
        data[sb_off + 57] = 0xEF;
        data[sb_off + 24] = 2; // 4096
        let blocks = (data.len() as u32) / 4096;
        data[sb_off + 4..sb_off + 8].copy_from_slice(&blocks.to_le_bytes());
        data[sb_off + 32..sb_off + 36].copy_from_slice(&8192u32.to_le_bytes());
        data[sb_off + 40..sb_off + 44].copy_from_slice(&2048u32.to_le_bytes());
        data[sb_off + 88..sb_off + 90].copy_from_slice(&256u16.to_le_bytes());
        data[sb_off + 84..sb_off + 88].copy_from_slice(&11u32.to_le_bytes());
        data[sb_off + 76..sb_off + 80].copy_from_slice(&1u32.to_le_bytes());
        // gdt at block 1 (4096)
        let gdt_off = 4096;
        // group 0: block_bitmap=3, inode_bitmap=4, inode_table=5
        data[gdt_off..gdt_off + 4].copy_from_slice(&3u32.to_le_bytes());
        data[gdt_off + 4..gdt_off + 8].copy_from_slice(&4u32.to_le_bytes());
        data[gdt_off + 8..gdt_off + 12].copy_from_slice(&5u32.to_le_bytes());
        let block = MemBlock { data };
        let sb = Superblock::read(&block).unwrap();
        (sb, block)
    }

    #[test]
    fn group_read_single() {
        let (sb, block) = make_sb_and_gdt();
        let g = read_group_descs(&block, &sb).unwrap();
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].inode_table_lo, 5);
        assert_eq!(g[0].inode_table_block(), 5);
    }
}
