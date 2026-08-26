use linfs_core::block::Block;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsType {
    Ext4,
    Xfs,
    Btrfs,
    F2fs,
    Unknown,
}

pub fn probe_fs(block: &dyn Block) -> FsType {
    let mut buf = vec![0u8; 0x20000 + 4096];
    // ext4 super at 1024, magic 0xEF53 at offset 56
    if block.read_at(1024, &mut buf[..2048]).is_ok() {
        let magic = u16::from_le_bytes([buf[56], buf[57]]);
        if magic == 0xEF53 {
            return FsType::Ext4;
        }
    }
    // xfs at 0, magic "XFSB"
    if block.read_at(0, &mut buf[..512]).is_ok() && &buf[0..4] == b"XFSB" {
        return FsType::Xfs;
    }
    // btrfs at 0x10000, magic "_BHRfS_M" at 64
    if block.read_at(0x10000, &mut buf[..4096]).is_ok() && &buf[64..72] == b"_BHRfS_M" {
        return FsType::Btrfs;
    }
    // f2fs at 0, magic 0xF2F52010 LE at 0
    if block.read_at(0, &mut buf[..512]).is_ok() {
        let m = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        if m == 0xF2F52010 {
            return FsType::F2fs;
        }
    }
    FsType::Unknown
}
