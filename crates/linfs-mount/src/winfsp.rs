// TODO band 203: winfsp-rs bridge — impl winfsp::Filesystem for LinfsFs
pub struct Mount;
impl Mount {
    pub fn new(
        _fs: std::sync::Arc<dyn linfs_core::fs::FileSystem>,
        _drive: &str,
    ) -> linfs_core::Result<Self> {
        Err(linfs_core::Error::Unsupported(
            "WinFSP mount not yet implemented (band 203)".into(),
        ))
    }
    pub fn unmount(self) -> linfs_core::Result<()> {
        Ok(())
    }
}
