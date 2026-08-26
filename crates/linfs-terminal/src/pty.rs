// TODO band 204: ConPTY via CreatePseudoConsole + CreateProcessW
pub struct Pty;
impl Pty {
    pub fn spawn(_shell: &str, _cols: u16, _rows: u16) -> linfs_core::Result<Self> {
        Err(linfs_core::Error::Unsupported(
            "Pty::spawn not yet implemented (band 204)".into(),
        ))
    }
}
