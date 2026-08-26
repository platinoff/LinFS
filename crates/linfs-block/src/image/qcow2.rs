/// Band 211: qcow2/vhd/vhdx ro parser stub — signature check only for MVP.
/// Full cluster/L1/L2/refcount parsing stretch.
pub const QCOW2_MAGIC: u32 = 0x514649fb; // "QFI\xfb" be32

pub fn is_qcow2(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }
    u32::from_be_bytes([data[0], data[1], data[2], data[3]]) == QCOW2_MAGIC
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn qcow2_detect() {
        let mut hdr = [0u8; 4];
        hdr.copy_from_slice(&QCOW2_MAGIC.to_be_bytes());
        assert!(is_qcow2(&hdr));
        assert!(!is_qcow2(&[0, 0, 0, 0]));
    }
}
