// TODO band 203: axum fallback server exposing /api/fs/* over 127.0.0.1:9998
pub async fn serve(
    _fs: std::sync::Arc<dyn linfs_core::fs::FileSystem>,
    _addr: &str,
) -> linfs_core::Result<()> {
    Err(linfs_core::Error::Unsupported(
        "fallback serve not yet implemented (band 203)".into(),
    ))
}
