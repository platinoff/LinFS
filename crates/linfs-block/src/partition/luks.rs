// TODO band 207: LUKS2 JSON header parse + argon2id + aes-xts-plain64
pub fn parse_header(_hdr: &[u8]) -> linfs_core::Result<LuksInfo> {
    Err(linfs_core::Error::Unsupported(
        "LUKS2 not yet implemented (band 207)".into(),
    ))
}
#[derive(Debug)]
pub struct LuksInfo {
    pub version: u32,
}
