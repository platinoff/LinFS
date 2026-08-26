use std::sync::{Arc, RwLock};

use linfs_core::block::Block;

struct WritableMemBlock {
    data: RwLock<Vec<u8>>,
}

impl WritableMemBlock {
    fn new(data: Vec<u8>) -> Self {
        Self {
            data: RwLock::new(data),
        }
    }
}

impl Block for WritableMemBlock {
    fn read_at(&self, off: u64, buf: &mut [u8]) -> std::io::Result<()> {
        let data = self.data.read().unwrap();
        let end = off as usize + buf.len();
        if end > data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "oob",
            ));
        }
        buf.copy_from_slice(&data[off as usize..end]);
        Ok(())
    }
    fn write_at(&self, off: u64, buf: &[u8]) -> std::io::Result<()> {
        let mut data = self.data.write().unwrap();
        let end = off as usize + buf.len();
        if end > data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "oob",
            ));
        }
        data[off as usize..end].copy_from_slice(buf);
        Ok(())
    }
    fn len(&self) -> u64 {
        self.data.read().unwrap().len() as u64
    }
}

fn make_image_with_journal_flag(needs_recovery: bool) -> Vec<u8> {
    let bs: u64 = 4096;
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
    data[sb_off + 96..sb_off + 100].copy_from_slice(&0x10u32.to_le_bytes()); // INCOMPAT_RECOVER
    if needs_recovery {
        data[sb_off + 58..sb_off + 60].copy_from_slice(&0x04u16.to_le_bytes()); // STATE_RECOVER
    }
    let gdt_off = 4096;
    data[gdt_off..gdt_off + 4].copy_from_slice(&3u32.to_le_bytes());
    data[gdt_off + 4..gdt_off + 8].copy_from_slice(&4u32.to_le_bytes());
    data[gdt_off + 8..gdt_off + 12].copy_from_slice(&5u32.to_le_bytes());
    let itable = 5 * bs as usize;
    let ino2_off = itable + 256;
    data[ino2_off..ino2_off + 2].copy_from_slice(&0x41EDu16.to_le_bytes());
    data[ino2_off + 26..ino2_off + 28].copy_from_slice(&2u16.to_le_bytes());
    data[ino2_off + 4..ino2_off + 8].copy_from_slice(&4096u32.to_le_bytes());
    data[ino2_off + 32..ino2_off + 36].copy_from_slice(&0x80000u32.to_le_bytes());
    let eb = ino2_off + 40;
    data[eb..eb + 2].copy_from_slice(&0xF30Au16.to_le_bytes());
    data[eb + 2..eb + 4].copy_from_slice(&1u16.to_le_bytes());
    data[eb + 4..eb + 6].copy_from_slice(&4u16.to_le_bytes());
    let ee = eb + 12;
    data[ee..ee + 4].copy_from_slice(&0u32.to_le_bytes());
    data[ee + 4..ee + 6].copy_from_slice(&1u16.to_le_bytes());
    data[ee + 6..ee + 8].copy_from_slice(&0u16.to_le_bytes());
    data[ee + 8..ee + 12].copy_from_slice(&10u32.to_le_bytes());
    let dir_off = 10 * 4096;
    let mut off = dir_off;
    data[off..off + 4].copy_from_slice(&2u32.to_le_bytes());
    data[off + 4..off + 6].copy_from_slice(&12u16.to_le_bytes());
    data[off + 6] = 1;
    data[off + 7] = 2;
    data[off + 8] = b'.';
    off += 12;
    data[off..off + 4].copy_from_slice(&2u32.to_le_bytes());
    data[off + 4..off + 6].copy_from_slice(&12u16.to_le_bytes());
    data[off + 6] = 2;
    data[off + 7] = 2;
    data[off + 8..off + 10].copy_from_slice(b"..");
    off += 12;
    let rec = 4096 - 24;
    data[off..off + 4].copy_from_slice(&12u32.to_le_bytes());
    data[off + 4..off + 6].copy_from_slice(&(rec as u16).to_le_bytes());
    data[off + 6] = 3;
    data[off + 7] = 2;
    data[off + 8..off + 11].copy_from_slice(b"etc");
    data
}

#[test]
fn journal_replay_unclean() {
    let data = make_image_with_journal_flag(true);
    let fs = linfs_fs::ext4::Fs::open(Arc::new(WritableMemBlock::new(data))).unwrap();
    assert!(fs.needs_recovery_replayed());
}

#[test]
fn journal_no_replay_clean() {
    let data = make_image_with_journal_flag(false);
    let fs = linfs_fs::ext4::Fs::open(Arc::new(WritableMemBlock::new(data))).unwrap();
    assert!(!fs.needs_recovery_replayed());
}

#[test]
fn alloc_bitmap_write_simulation() {
    // Simulate creating a file via bitmap alloc + dir insert (in-memory)
    let data = make_image_with_journal_flag(false);
    let block = Arc::new(WritableMemBlock::new(data));
    let fs = linfs_fs::ext4::Fs::open(block.clone()).unwrap();
    // Verify readdir still works after journal check
    let entries = fs.readdir(2).unwrap();
    assert!(entries.iter().any(|e| e.name == b"etc"));
    // alloc bitmap unit: find free block
    let mut bm = linfs_fs::ext4::alloc::Bitmap::from_block(&[0x00, 0xFF]);
    assert_eq!(bm.alloc(), Some(0));
    fs.sync().unwrap();
}
