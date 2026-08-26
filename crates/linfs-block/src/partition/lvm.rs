/// Scan for LVM2 PV LABELONE at sector 1 (512 bytes offset 512).
pub fn scan(block: &dyn linfs_core::block::Block) -> Vec<String> {
    let mut buf = [0u8; 512];
    if block.read_at(512, &mut buf).is_err() {
        return vec![];
    }
    if &buf[0..8] == b"LABELONE" {
        // PV UUID at offset 32..64
        let uuid = String::from_utf8_lossy(&buf[32..64])
            .trim_matches(|c| c == '\0' || c == ' ')
            .to_string();
        if uuid.is_empty() {
            vec!["pv-unknown".to_string()]
        } else {
            vec![uuid]
        }
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linfs_core::block::Block;

    struct MemBlock(Vec<u8>);
    impl Block for MemBlock {
        fn read_at(&self, off: u64, buf: &mut [u8]) -> std::io::Result<()> {
            buf.copy_from_slice(&self.0[off as usize..off as usize + buf.len()]);
            Ok(())
        }
        fn write_at(&self, _off: u64, _buf: &[u8]) -> std::io::Result<()> {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "ro",
            ))
        }
        fn len(&self) -> u64 {
            self.0.len() as u64
        }
    }

    #[test]
    fn lvm_labelone_detect() {
        let mut data = vec![0u8; 4096];
        data[512..520].copy_from_slice(b"LABELONE");
        data[512 + 32..512 + 40].copy_from_slice(b"abc123  ");
        let block = MemBlock(data);
        let v = scan(&block);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn lvm_no_label() {
        let data = vec![0u8; 4096];
        let block = MemBlock(data);
        assert!(scan(&block).is_empty());
    }
}
