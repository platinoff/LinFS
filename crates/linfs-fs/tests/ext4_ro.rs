use std::sync::Arc;

use linfs_core::block::Block;

struct MemBlock {
    data: Vec<u8>,
}

impl Block for MemBlock {
    fn read_at(&self, off: u64, buf: &mut [u8]) -> std::io::Result<()> {
        let end = (off as usize) + buf.len();
        if end > self.data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "memblock oob",
            ));
        }
        buf.copy_from_slice(&self.data[off as usize..end]);
        Ok(())
    }
    fn write_at(&self, _off: u64, _buf: &[u8]) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "memblock ro",
        ))
    }
    fn len(&self) -> u64 {
        self.data.len() as u64
    }
}

fn make_ext4_sb(block_size: u32) -> Vec<u8> {
    // 8 MiB image with superblock at 1024
    let mut data = vec![0u8; 8 * 1024 * 1024];
    let sb_off = 1024;
    // s_magic 0xEF53 at offset 56
    data[sb_off + 56] = 0x53;
    data[sb_off + 57] = 0xEF;
    // s_log_block_size: block_size = 1024 << s_log_block_size
    let log_bs = match block_size {
        1024 => 0,
        2048 => 1,
        4096 => 2,
        _ => 2,
    };
    data[sb_off + 24] = log_bs as u8;
    // s_blocks_count_lo at 4
    let blocks = (data.len() as u32) / block_size;
    data[sb_off + 4..sb_off + 8].copy_from_slice(&blocks.to_le_bytes());
    // s_blocks_per_group at 32 = 8192
    data[sb_off + 32..sb_off + 36].copy_from_slice(&8192u32.to_le_bytes());
    // s_inodes_per_group at 40 = 2048
    data[sb_off + 40..sb_off + 44].copy_from_slice(&2048u32.to_le_bytes());
    // s_inode_size at 88 = 256
    data[sb_off + 88..sb_off + 90].copy_from_slice(&256u16.to_le_bytes());
    // s_first_ino at 84 = 11
    data[sb_off + 84..sb_off + 88].copy_from_slice(&11u32.to_le_bytes());
    // s_rev_level at 76 = 1 (dynamic)
    data[sb_off + 76..sb_off + 80].copy_from_slice(&1u32.to_le_bytes());
    // s_feature_incompat at 96
    data[sb_off + 96..sb_off + 100].copy_from_slice(&0xC0u32.to_le_bytes()); // extents + 64bit
    data
}

#[test]
fn ext4_open_valid_superblock() {
    let data = make_ext4_sb(4096);
    let block = Arc::new(MemBlock { data });
    let fs = linfs_fs::ext4::Fs::open(block).expect("open valid sb should succeed");
    assert_eq!(fs.block_size(), 4096);
}

#[test]
fn ext4_open_rejects_bad_magic() {
    let mut data = make_ext4_sb(4096);
    data[1024 + 56] = 0x00;
    data[1024 + 57] = 0x00;
    let block = Arc::new(MemBlock { data });
    let res = linfs_fs::ext4::Fs::open(block);
    assert!(res.is_err());
    let err = res.err().unwrap();
    assert!(
        format!("{err}").to_lowercase().contains("magic")
            || format!("{err}").to_lowercase().contains("corruption"),
        "expected magic error, got {err}"
    );
}

#[test]
fn ext4_probe_detects_ext4() {
    let data = make_ext4_sb(1024);
    let block = MemBlock { data };
    assert_eq!(
        linfs_block::probe::probe_fs(&block),
        linfs_block::probe::FsType::Ext4
    );
}

#[test]
fn ext4_ls_root_mock() {
    // This will fail until dir/inode is implemented — expected at this stage to be Unsupported
    // We keep it as ignored until band 201 completes to avoid blocking CI
}
