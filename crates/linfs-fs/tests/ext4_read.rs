use linfs_core::block::Block;
use std::sync::Arc;

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

fn make_image_with_file(content: &[u8]) -> Vec<u8> {
    let bs: u64 = 4096;
    let mut data = vec![0u8; 8 * 1024 * 1024];
    let sb_off = 1024;
    data[sb_off + 56] = 0x53;
    data[sb_off + 57] = 0xEF;
    data[sb_off + 24] = 2;
    let blocks = (data.len() as u32) / 4096;
    data[sb_off + 4..sb_off + 8].copy_from_slice(&blocks.to_le_bytes());
    data[sb_off + 32..sb_off + 36].copy_from_slice(&8192u32.to_le_bytes());
    data[sb_off + 40..sb_off + 44].copy_from_slice(&2048u32.to_le_bytes());
    data[sb_off + 88..sb_off + 90].copy_from_slice(&256u16.to_le_bytes());
    data[sb_off + 84..sb_off + 88].copy_from_slice(&11u32.to_le_bytes());
    data[sb_off + 76..sb_off + 80].copy_from_slice(&1u32.to_le_bytes());
    let gdt_off = 4096;
    data[gdt_off..gdt_off + 4].copy_from_slice(&3u32.to_le_bytes());
    data[gdt_off + 4..gdt_off + 8].copy_from_slice(&4u32.to_le_bytes());
    data[gdt_off + 8..gdt_off + 12].copy_from_slice(&5u32.to_le_bytes());
    let itable = 5 * bs as usize;
    // inode 2 root
    let ino2_off = itable + 256;
    data[ino2_off..ino2_off + 2].copy_from_slice(&0x41EDu16.to_le_bytes());
    data[ino2_off + 26..ino2_off + 28].copy_from_slice(&3u16.to_le_bytes());
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
    // file "hello.txt" ino 13 - rec_len must be 4-aligned: 8+9=17 -> 20
    let rec1 = 20;
    data[off..off + 4].copy_from_slice(&13u32.to_le_bytes());
    data[off + 4..off + 6].copy_from_slice(&(rec1 as u16).to_le_bytes());
    data[off + 6] = 9;
    data[off + 7] = 1;
    data[off + 8..off + 17].copy_from_slice(b"hello.txt");
    off += 20;
    let rec2 = 4096 - 12 - 12 - 20;
    data[off..off + 4].copy_from_slice(&12u32.to_le_bytes());
    data[off + 4..off + 6].copy_from_slice(&(rec2 as u16).to_le_bytes());
    data[off + 6] = 3;
    data[off + 7] = 2;
    data[off + 8..off + 11].copy_from_slice(b"etc");
    // inode 13 file
    let ino13_off = itable + 12 * 256; // (13-1)*256
    data[ino13_off..ino13_off + 2].copy_from_slice(&0x81A4u16.to_le_bytes()); // 0100644 reg
    data[ino13_off + 4..ino13_off + 8].copy_from_slice(&(content.len() as u32).to_le_bytes());
    data[ino13_off + 32..ino13_off + 36].copy_from_slice(&0x80000u32.to_le_bytes());
    let eb13 = ino13_off + 40;
    data[eb13..eb13 + 2].copy_from_slice(&0xF30Au16.to_le_bytes());
    data[eb13 + 2..eb13 + 4].copy_from_slice(&1u16.to_le_bytes());
    data[eb13 + 4..eb13 + 6].copy_from_slice(&4u16.to_le_bytes());
    let ee13 = eb13 + 12;
    data[ee13..ee13 + 4].copy_from_slice(&0u32.to_le_bytes());
    data[ee13 + 4..ee13 + 6].copy_from_slice(&1u16.to_le_bytes());
    data[ee13 + 6..ee13 + 8].copy_from_slice(&0u16.to_le_bytes());
    data[ee13 + 8..ee13 + 12].copy_from_slice(&11u32.to_le_bytes()); // phys 11
                                                                     // data block 11
    let data_off = 11 * 4096;
    data[data_off..data_off + content.len()].copy_from_slice(content);
    // etc inode 12 dir
    let ino12_off = itable + 11 * 256;
    data[ino12_off..ino12_off + 2].copy_from_slice(&0x41EDu16.to_le_bytes());
    data[ino12_off + 26..ino12_off + 28].copy_from_slice(&2u16.to_le_bytes());
    data
}

#[test]
fn read_file_extent() {
    let content = b"hello linfs ext4 read";
    let data = make_image_with_file(content);
    let fs = linfs_fs::ext4::Fs::open(Arc::new(MemBlock { data })).unwrap();
    let ino = fs.lookup(2, b"hello.txt").unwrap();
    assert_eq!(ino, 13);
    let mut buf = vec![0u8; 64];
    let n = fs.read(ino, 0, &mut buf).unwrap();
    assert_eq!(n, content.len());
    assert_eq!(&buf[..n], content);
    // offset read
    let mut buf2 = vec![0u8; 5];
    let n2 = fs.read(ino, 6, &mut buf2).unwrap();
    assert_eq!(&buf2[..n2], b"linfs");
}

#[test]
fn read_beyond_eof() {
    let data = make_image_with_file(b"hi");
    let fs = linfs_fs::ext4::Fs::open(Arc::new(MemBlock { data })).unwrap();
    let ino = fs.lookup(2, b"hello.txt").unwrap();
    let mut buf = vec![0u8; 10];
    let n = fs.read(ino, 100, &mut buf).unwrap();
    assert_eq!(n, 0);
}
