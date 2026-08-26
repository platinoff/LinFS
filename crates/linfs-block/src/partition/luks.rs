#[derive(Debug)]
pub struct LuksInfo {
    pub version: u32,
    pub label: String,
}

pub fn parse_header(hdr: &[u8]) -> linfs_core::Result<LuksInfo> {
    if hdr.len() < 4096 {
        return Err(linfs_core::Error::Corruption(
            "luks header too short".into(),
        ));
    }
    if &hdr[0..6] != b"LUKS\xba\xbe" {
        return Err(linfs_core::Error::Corruption("luks magic mismatch".into()));
    }
    let version = u16::from_be_bytes([hdr[6], hdr[7]]) as u32;
    if version != 1 && version != 2 {
        return Err(linfs_core::Error::Corruption(format!(
            "luks version {version}"
        )));
    }
    // Label at 168..200 (LUKS2)
    let label_bytes = &hdr[168..200];
    let end = label_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(label_bytes.len());
    let label = String::from_utf8_lossy(&label_bytes[..end]).to_string();
    Ok(LuksInfo { version, label })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn luks2_parse() {
        let mut hdr = vec![0u8; 4096];
        hdr[0..6].copy_from_slice(b"LUKS\xba\xbe");
        hdr[6..8].copy_from_slice(&2u16.to_be_bytes());
        hdr[168..171].copy_from_slice(b"ssd");
        let info = parse_header(&hdr).unwrap();
        assert_eq!(info.version, 2);
        assert_eq!(info.label, "ssd");
    }
}
