#[derive(Debug, Clone)]
pub struct DirEntry {
    pub inode: u32,
    pub rec_len: u16,
    pub name_len: u8,
    pub file_type: u8,
    pub name: Vec<u8>,
}

/// Parse linear `ext4_dir_entry_2` entries from a block (block_size bytes).
pub fn parse_dir_block(block: &[u8]) -> linfs_core::Result<Vec<DirEntry>> {
    let mut out = Vec::new();
    let mut off = 0usize;
    while off + 8 <= block.len() {
        let inode =
            u32::from_le_bytes([block[off], block[off + 1], block[off + 2], block[off + 3]]);
        let rec_len = u16::from_le_bytes([block[off + 4], block[off + 5]]);
        let name_len = block[off + 6];
        let file_type = block[off + 7];
        if rec_len == 0 {
            return Err(linfs_core::Error::Corruption("dir rec_len 0".into()));
        }
        if rec_len < 8 {
            return Err(linfs_core::Error::Corruption(format!(
                "dir rec_len too short {rec_len}"
            )));
        }
        if off + rec_len as usize > block.len() {
            return Err(linfs_core::Error::Corruption("dir entry past block".into()));
        }
        if inode == 0 {
            off += rec_len as usize;
            continue;
        }
        if off + 8 + name_len as usize > block.len() {
            return Err(linfs_core::Error::Corruption("dir name oob".into()));
        }
        let name = block[off + 8..off + 8 + name_len as usize].to_vec();
        out.push(DirEntry {
            inode,
            rec_len,
            name_len,
            file_type,
            name,
        });
        if rec_len as usize == 0 {
            break;
        }
        off += rec_len as usize;
        if off >= block.len() {
            break;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(inode: u32, name: &[u8], file_type: u8, rec_len: u16) -> Vec<u8> {
        let mut v = vec![0u8; rec_len as usize];
        v[0..4].copy_from_slice(&inode.to_le_bytes());
        v[4..6].copy_from_slice(&rec_len.to_le_bytes());
        v[6] = name.len() as u8;
        v[7] = file_type;
        v[8..8 + name.len()].copy_from_slice(name);
        v
    }

    #[test]
    fn dir_parse_single() {
        let e = make_entry(12, b"etc", 2, 16);
        let entries = parse_dir_block(&e).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, b"etc");
        assert_eq!(entries[0].inode, 12);
    }

    #[test]
    fn dir_parse_two_plus_dot() {
        let mut block = Vec::new();
        block.extend(make_entry(2, b".", 2, 12));
        block.extend(make_entry(2, b"..", 2, 12));
        // last entry fills rest of 4096 block
        let last_len = 4096 - 24;
        block.extend(make_entry(12, b"etc", 2, last_len as u16));
        block.resize(4096, 0);
        let entries = parse_dir_block(&block).unwrap();
        assert!(entries.iter().any(|e| e.name == b"etc"));
    }

    #[test]
    fn dir_rejects_zero_rec_len() {
        let mut block = vec![0u8; 32];
        block[0..4].copy_from_slice(&12u32.to_le_bytes());
        block[4..6].copy_from_slice(&0u16.to_le_bytes());
        assert!(parse_dir_block(&block).is_err());
    }
}
