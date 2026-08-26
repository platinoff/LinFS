#[derive(Debug, Clone)]
pub struct Inode {
    pub mode: u16,
    pub uid: u16,
    pub size_lo: u32,
    pub atime: u32,
    pub ctime: u32,
    pub mtime: u32,
    pub dtime: u32,
    pub gid: u16,
    pub links_count: u16,
    pub blocks_lo: u32,
    pub flags: u32,
    pub size_hi: u32,
    pub block: [u32; 15],
    pub generation: u32,
    pub file_acl_lo: u32,
    pub size_high: u64,
}

impl Inode {
    pub fn file_type(&self) -> u8 {
        ((self.mode >> 12) & 0xF) as u8
    }
    pub fn is_dir(&self) -> bool {
        (self.mode & 0xF000) == 0x4000
    }
    pub fn is_reg(&self) -> bool {
        (self.mode & 0xF000) == 0x8000
    }
    pub fn size(&self) -> u64 {
        (self.size_lo as u64) | ((self.size_hi as u64) << 32)
    }
}

pub fn parse_inode(buf: &[u8], inode_size: u16) -> linfs_core::Result<Inode> {
    if buf.len() < 128 {
        return Err(linfs_core::Error::Corruption("inode buf too short".into()));
    }
    if buf.len() < inode_size as usize {
        return Err(linfs_core::Error::Corruption("inode truncated".into()));
    }
    let mode = u16::from_le_bytes([buf[0], buf[1]]);
    let uid = u16::from_le_bytes([buf[2], buf[3]]);
    let size_lo = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
    let atime = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
    let ctime = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
    let mtime = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);
    let dtime = u32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]);
    let gid = u16::from_le_bytes([buf[24], buf[25]]);
    let links_count = u16::from_le_bytes([buf[26], buf[27]]);
    let blocks_lo = u32::from_le_bytes([buf[28], buf[29], buf[30], buf[31]]);
    let flags = u32::from_le_bytes([buf[32], buf[33], buf[34], buf[35]]);
    let mut block = [0u32; 15];
    for (i, slot) in block.iter_mut().enumerate() {
        let off = 40 + i * 4;
        *slot = u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]);
    }
    let generation = u32::from_le_bytes([buf[100], buf[101], buf[102], buf[103]]);
    let file_acl_lo = u32::from_le_bytes([buf[104], buf[105], buf[106], buf[107]]);
    let size_hi = u32::from_le_bytes([buf[108], buf[109], buf[110], buf[111]]);
    let size_high = (size_lo as u64) | ((size_hi as u64) << 32);
    let _ = size_high;

    Ok(Inode {
        mode,
        uid,
        size_lo,
        atime,
        ctime,
        mtime,
        dtime,
        gid,
        links_count,
        blocks_lo,
        flags,
        size_hi,
        block,
        generation,
        file_acl_lo,
        size_high,
    })
}

pub fn inode_offset(
    ino: u32,
    sb: &super::superblock::Superblock,
    gdt: &super::group::GroupDesc,
) -> (u64, usize) {
    // inode number is 1-indexed; 0 is invalid
    let inodes_per_group = sb.inodes_per_group as u64;
    let group = (ino as u64 - 1) / inodes_per_group;
    let index = (ino as u64 - 1) % inodes_per_group;
    let table_block = gdt.inode_table_block();
    let block_size = sb.block_size as u64;
    let inode_size = sb.inode_size as u64;
    let byte_off = table_block * block_size + index * inode_size;
    // For now assume single group (gdt 0)
    let _ = group;
    (byte_off, inode_size as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inode_parse_dir() {
        let mut buf = vec![0u8; 256];
        buf[0..2].copy_from_slice(&0x41EDu16.to_le_bytes()); // 0040777 dir
        buf[4..8].copy_from_slice(&4096u32.to_le_bytes());
        let ino = parse_inode(&buf, 256).unwrap();
        assert!(ino.is_dir());
        assert_eq!(ino.size(), 4096);
    }
}
