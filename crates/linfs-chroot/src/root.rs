/// Chroot path translator — clamps `..` at `/`, resolves host binds.
// TODO band 204
pub struct Root;
impl Root {
    pub fn new(_fs: std::sync::Arc<dyn linfs_core::fs::FileSystem>) -> Self {
        Self
    }
    pub fn resolve(&self, _cwd: &str, _path: &str) -> linfs_core::Result<String> {
        Err(linfs_core::Error::Unsupported(
            "Root::resolve not yet implemented (band 204)".into(),
        ))
    }
}
