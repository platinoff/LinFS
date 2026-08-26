use std::sync::Arc;

use linfs_core::block::Block;

struct MemBlock {
    data: Vec<u8>,
}

impl Block for MemBlock {
    fn read_at(&self, off: u64, buf: &mut [u8]) -> std::io::Result<()> {
        let end = off as usize + buf.len();
        if end > self.data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "oob",
            ));
        }
        buf.copy_from_slice(&self.data[off as usize..end]);
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

fn make_image_with_root_dir() -> Vec<u8> {
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
    // GDT at block 1 (4096)
    let gdt_off = 4096;
    data[gdt_off..gdt_off + 4].copy_from_slice(&3u32.to_le_bytes()); // bbitmap
    data[gdt_off + 4..gdt_off + 8].copy_from_slice(&4u32.to_le_bytes()); // ibitmap
    data[gdt_off + 8..gdt_off + 12].copy_from_slice(&5u32.to_le_bytes()); // itable at block 5

    // Inode table at block 5 (20480)
    let itable = 5 * bs as usize;
    // inode 2 (root) at index 1: offset 1*256
    let ino2_off = itable + 256;
    data[ino2_off..ino2_off + 2].copy_from_slice(&0x41EDu16.to_le_bytes()); // dir 0777
    data[ino2_off + 26..ino2_off + 28].copy_from_slice(&2u16.to_le_bytes()); // links 2
    data[ino2_off + 4..ino2_off + 8].copy_from_slice(&4096u32.to_le_bytes()); // size
    data[ino2_off + 28..ino2_off + 32].copy_from_slice(&1u32.to_le_bytes()); // blocks (1*8)
    data[ino2_off + 32..ino2_off + 36].copy_from_slice(&0x80000u32.to_le_bytes()); // flags EXTENTS
                                                                                   // extent tree: header + 1 extent -> phys block 10
    let eb = ino2_off + 40;
    data[eb..eb + 2].copy_from_slice(&0xF30Au16.to_le_bytes());
    data[eb + 2..eb + 4].copy_from_slice(&1u16.to_le_bytes()); // entries 1
    data[eb + 4..eb + 6].copy_from_slice(&4u16.to_le_bytes()); // max 4
    data[eb + 6..eb + 8].copy_from_slice(&0u16.to_le_bytes()); // depth 0
                                                               // extent at 12 bytes offset
    let ee = eb + 12;
    data[ee..ee + 4].copy_from_slice(&0u32.to_le_bytes()); // logical 0
    data[ee + 4..ee + 6].copy_from_slice(&1u16.to_le_bytes()); // len 1
    data[ee + 6..ee + 8].copy_from_slice(&0u16.to_le_bytes()); // hi
    data[ee + 8..ee + 12].copy_from_slice(&10u32.to_le_bytes()); // lo 10

    // Dir block at phys 10 (40960)
    let dir_off = 10 * 4096;
    let mut off = dir_off;
    // "."
    data[off..off + 4].copy_from_slice(&2u32.to_le_bytes());
    data[off + 4..off + 6].copy_from_slice(&12u16.to_le_bytes());
    data[off + 6] = 1;
    data[off + 7] = 2;
    data[off + 8] = b'.';
    off += 12;
    // ".."
    data[off..off + 4].copy_from_slice(&2u32.to_le_bytes());
    data[off + 4..off + 6].copy_from_slice(&12u16.to_le_bytes());
    data[off + 6] = 2;
    data[off + 7] = 2;
    data[off + 8..off + 10].copy_from_slice(b"..");
    off += 12;
    // "etc" -> ino 12, rec_len = rest of block (4096-24)
    let rec = 4096 - 24;
    data[off..off + 4].copy_from_slice(&12u32.to_le_bytes());
    data[off + 4..off + 6].copy_from_slice(&(rec as u16).to_le_bytes());
    data[off + 6] = 3;
    data[off + 7] = 2;
    data[off + 8..off + 11].copy_from_slice(b"etc");

    // inode 12 (etc) at index 11: (12-1)*256 = 2816
    let ino12_off = itable + 11 * 256;
    data[ino12_off..ino12_off + 2].copy_from_slice(&0x41EDu16.to_le_bytes());
    data[ino12_off + 26..ino12_off + 28].copy_from_slice(&2u16.to_le_bytes());

    data
}

#[test]
fn readdir_root_contains_etc() {
    let data = make_image_with_root_dir();
    let fs = linfs_fs::ext4::Fs::open(Arc::new(MemBlock { data })).unwrap();
    let entries = fs.readdir(2).unwrap();
    assert!(entries.iter().any(|e| e.name == b"etc" && e.inode == 12));
    assert!(entries.iter().any(|e| e.name == b"." && e.inode == 2));
}

#[test]
fn lookup_etc() {
    let data = make_image_with_root_dir();
    let fs = linfs_fs::ext4::Fs::open(Arc::new(MemBlock { data })).unwrap();
    let ino = fs.lookup(2, b"etc").unwrap();
    assert_eq!(ino, 12);
    let attr = fs.getattr(ino).unwrap();
    assert!(attr.is_dir());
}
