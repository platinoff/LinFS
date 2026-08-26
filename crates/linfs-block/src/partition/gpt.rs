use super::Partition;

pub fn parse_gpt(hdr: &[u8]) -> linfs_core::Result<Vec<Partition>> {
    if hdr.len() < 512 {
        return Err(linfs_core::Error::Corruption("GPT header too short".into()));
    }
    if &hdr[0..8] != b"EFI PART" {
        return Err(linfs_core::Error::Corruption("GPT magic mismatch".into()));
    }
    let header_size = u32::from_le_bytes([hdr[12], hdr[13], hdr[14], hdr[15]]) as usize;
    let crc_stored = u32::from_le_bytes([hdr[16], hdr[17], hdr[18], hdr[19]]);
    let mut tmp = hdr[0..header_size].to_vec();
    tmp[16..20].copy_from_slice(&[0, 0, 0, 0]);
    let crc_calc = crc32fast::hash(&tmp);
    if crc_calc != crc_stored {
        return Err(linfs_core::Error::Corruption(format!(
            "GPT CRC mismatch {crc_calc:08x} != {crc_stored:08x}"
        )));
    }
    // TODO: parse entries array (LBA from hdr[72..80], count from hdr[80..84])
    Ok(vec![])
}
