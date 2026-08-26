/// ext4 xattr — inline in inode extra_isize + external block.
/// Band 201: parse `ext4_xattr_header` + entries (name/value) for `user.*`.
#[derive(Debug, Clone)]
pub struct XattrEntry {
    pub name_index: u8,
    pub name: Vec<u8>,
    pub value: Vec<u8>,
}

pub const XATTR_MAGIC: u32 = 0xEA020000;

#[derive(Debug, Clone)]
pub struct XattrHeader {
    pub magic: u32,
    pub blocks: u32,
}

pub fn parse_xattr_block(buf: &[u8]) -> linfs_core::Result<Vec<XattrEntry>> {
    if buf.len() < 4 {
        return Err(linfs_core::Error::Corruption(
            "xattr block too short".into(),
        ));
    }
    let magic = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    if magic != XATTR_MAGIC {
        return Err(linfs_core::Error::Corruption(format!(
            "xattr bad magic 0x{magic:08x}"
        )));
    }
    // Header is 32 bytes: magic, hash, etc. Entries start at offset 32.
    // Each entry: e_name_len(1) e_name_index(1) e_value_offs(2) e_value_block(4) e_value_size(4) e_hash(4) | name bytes
    let mut out = Vec::new();
    let mut off = 32usize;
    while off + 16 <= buf.len() {
        let name_len = buf[off] as usize;
        let name_index = buf[off + 1];
        let value_offs = u16::from_le_bytes([buf[off + 2], buf[off + 3]]) as usize;
        let value_size =
            u32::from_le_bytes([buf[off + 8], buf[off + 9], buf[off + 10], buf[off + 11]]) as usize;
        if name_len == 0 && name_index == 0 && value_size == 0 {
            // terminator (all zeros)
            break;
        }
        if off + 16 + name_len > buf.len() {
            break;
        }
        let name = buf[off + 16..off + 16 + name_len].to_vec();
        let value = if value_size > 0 && value_offs + value_size <= buf.len() {
            buf[value_offs..value_offs + value_size].to_vec()
        } else {
            Vec::new()
        };
        out.push(XattrEntry {
            name_index,
            name,
            value,
        });
        // entry is 16 bytes + padded name len to 4-byte alignment
        let entry_len = 16 + ((name_len + 3) & !3);
        off += entry_len;
        if off >= buf.len() {
            break;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xattr_empty_block() {
        let mut buf = vec![0u8; 4096];
        buf[0..4].copy_from_slice(&XATTR_MAGIC.to_le_bytes());
        let entries = parse_xattr_block(&buf).unwrap();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn xattr_single_user_entry() {
        let mut buf = vec![0u8; 4096];
        buf[0..4].copy_from_slice(&XATTR_MAGIC.to_le_bytes());
        // entry at 32: name "user.test" split? use index 1 = user
        let off = 32;
        buf[off] = 4; // name_len "test"
        buf[off + 1] = 1; // USER
        buf[off + 2..off + 4].copy_from_slice(&1024u16.to_le_bytes()); // value at 1024
        buf[off + 8..off + 12].copy_from_slice(&5u32.to_le_bytes()); // value size 5
        buf[off + 16..off + 20].copy_from_slice(b"test");
        buf[1024..1029].copy_from_slice(b"hello");
        let entries = parse_xattr_block(&buf).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, b"test");
        assert_eq!(entries[0].value, b"hello");
    }

    #[test]
    fn xattr_bad_magic() {
        let buf = vec![0u8; 4096];
        assert!(parse_xattr_block(&buf).is_err());
    }
}
