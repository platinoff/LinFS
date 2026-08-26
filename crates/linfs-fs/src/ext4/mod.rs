pub mod alloc;
pub mod dir;
pub mod extent;
pub mod group;
pub mod inode;
pub mod journal;
pub mod superblock;
pub mod xattr;

pub struct Fs;
impl Fs {
    pub fn open(_block: std::sync::Arc<dyn linfs_core::block::Block>) -> linfs_core::Result<Self> {
        Err(linfs_core::Error::Unsupported(
            "ext4 open not yet implemented (band 201)".into(),
        ))
    }
}
