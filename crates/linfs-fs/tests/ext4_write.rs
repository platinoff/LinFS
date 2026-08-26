use linfs_core::block::Block;
use std::sync::Arc;

struct WMem {
    data: std::sync::RwLock<Vec<u8>>,
}
impl WMem {
    fn new(data: Vec<u8>) -> Self {
        Self {
            data: std::sync::RwLock::new(data),
        }
    }
}
impl Block for WMem {
    fn read_at(&self, off: u64, buf: &mut [u8]) -> std::io::Result<()> {
        let d = self.data.read().unwrap();
        let end = off as usize + buf.len();
        if end > d.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "oob",
            ));
        }
        buf.copy_from_slice(&d[off as usize..end]);
        Ok(())
    }
    fn write_at(&self, off: u64, buf: &[u8]) -> std::io::Result<()> {
        let mut d = self.data.write().unwrap();
        let end = off as usize + buf.len();
        if end > d.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "oob",
            ));
        }
        d[off as usize..end].copy_from_slice(buf);
        Ok(())
    }
    fn len(&self) -> u64 {
        self.data.read().unwrap().len() as u64
    }
}

fn make_image() -> Vec<u8> {
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
    let ino12_off = itable + 11 * 256;
    data[ino12_off..ino12_off + 2].copy_from_slice(&0x41EDu16.to_le_bytes());
    data[ino12_off + 26..ino12_off + 28].copy_from_slice(&2u16.to_le_bytes());
    data
}

#[test]
fn create_write_read() {
    let data = make_image();
    let block = Arc::new(WMem::new(data));
    let fs = linfs_fs::ext4::Fs::open(block.clone()).unwrap();
    let ino = fs.create(2, b"hello.txt", 0o644).unwrap();
    assert!(ino >= 20);
    let n = fs.write_bytes(ino, 0, b"hello linfs 202").unwrap();
    assert_eq!(n, 15);
    fs.sync().unwrap();
    let mut buf = vec![0u8; 32];
    let nr = fs.read(ino, 0, &mut buf).unwrap();
    assert_eq!(&buf[..nr], b"hello linfs 202");
    // remount simulation: reopen Fs on same block
    let fs2 = linfs_fs::ext4::Fs::open(block).unwrap();
    let ino2 = fs2.lookup(2, b"hello.txt").unwrap();
    assert_eq!(ino2, ino);
    let mut buf2 = vec![0u8; 32];
    let nr2 = fs2.read(ino2, 0, &mut buf2).unwrap();
    assert_eq!(&buf2[..nr2], b"hello linfs 202");
}

#[test]
fn mkdir_rename_unlink() {
    let data = make_image();
    let block = Arc::new(WMem::new(data));
    let fs = linfs_fs::ext4::Fs::open(block).unwrap();
    let dino = fs.mkdir(2, b"mydir", 0o755).unwrap();
    let entries = fs.readdir(2).unwrap();
    assert!(entries.iter().any(|e| e.name == b"mydir"));
    let fino = fs.create(dino, b"inner.txt", 0o644).unwrap();
    fs.write_bytes(fino, 0, b"inner").unwrap();
    fs.rename(dino, b"inner.txt", 2, b"moved.txt").unwrap();
    assert!(fs.lookup(2, b"moved.txt").is_ok());
    assert!(fs.lookup(dino, b"inner.txt").is_err());
    fs.unlink(2, b"moved.txt").unwrap();
    assert!(fs.lookup(2, b"moved.txt").is_err());
}

#[test]
fn chmod() {
    let data = make_image();
    let block = Arc::new(WMem::new(data));
    let fs = linfs_fs::ext4::Fs::open(block).unwrap();
    let ino = fs.create(2, b"f.txt", 0o644).unwrap();
    fs.chmod(ino, 0o600).unwrap();
    let attr = fs.getattr(ino).unwrap();
    assert_eq!(attr.mode & 0o777, 0o600);
}
