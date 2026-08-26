pub mod alloc;
pub mod dir;
pub mod extent;
pub mod group;
pub mod inode;
pub mod journal;
pub mod superblock;
pub mod xattr;

use std::sync::Arc;

use linfs_core::block::Block;

pub struct Fs {
    block: Arc<dyn Block>,
    superblock: superblock::Superblock,
}

impl Fs {
    pub fn open(block: Arc<dyn Block>) -> linfs_core::Result<Self> {
        let sb = superblock::Superblock::read(&*block)?;
        Ok(Self {
            block,
            superblock: sb,
        })
    }

    pub fn block_size(&self) -> u32 {
        self.superblock.block_size
    }

    pub fn superblock(&self) -> &superblock::Superblock {
        &self.superblock
    }
}
