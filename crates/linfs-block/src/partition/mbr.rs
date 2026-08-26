use super::Partition;

pub fn parse_mbr(mbr: &[u8; 512]) -> linfs_core::Result<Vec<Partition>> {
    if mbr[510] != 0x55 || mbr[511] != 0xAA {
        return Err(linfs_core::Error::Corruption(
            "MBR signature missing".into(),
        ));
    }
    let mut out = Vec::new();
    for i in 0..4 {
        let off = 446 + i * 16;
        let ty = mbr[off + 4];
        if ty == 0 {
            continue;
        }
        let lba =
            u32::from_le_bytes([mbr[off + 8], mbr[off + 9], mbr[off + 10], mbr[off + 11]]) as u64;
        let sectors =
            u32::from_le_bytes([mbr[off + 12], mbr[off + 13], mbr[off + 14], mbr[off + 15]]) as u64;
        out.push(Partition {
            index: i as u32,
            offset: lba * 512,
            length: sectors * 512,
            ty,
            label: format!("mbr{i}"),
        });
    }
    Ok(out)
}
