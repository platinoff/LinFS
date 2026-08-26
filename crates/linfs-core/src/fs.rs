/// Ino = Linux inode number (1-indexed). 2 == root.
pub type Ino = u64;

#[derive(Debug, Clone)]
pub struct Attr {
    pub ino: Ino,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub nlink: u32,
    pub mtime: i64,
    pub is_dir: bool,
    pub is_symlink: bool,
}

#[derive(Debug, Clone)]
pub struct Dirent {
    pub ino: Ino,
    pub name: Vec<u8>,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub struct FsStat {
    pub blocks: u64,
    pub bfree: u64,
    pub bsize: u32,
    pub files: u64,
    pub ffree: u64,
}

pub trait FileSystem: Send + Sync {
    fn statfs(&self) -> FsStat;
    fn lookup(&self, parent: Ino, name: &[u8]) -> crate::Result<Ino>;
    fn getattr(&self, ino: Ino) -> crate::Result<Attr>;
    fn readdir(&self, ino: Ino) -> crate::Result<Vec<Dirent>>;
    fn read(&self, ino: Ino, off: u64, buf: &mut [u8]) -> crate::Result<usize>;
    fn write(&self, ino: Ino, off: u64, buf: &[u8]) -> crate::Result<usize>;
    fn create(&self, parent: Ino, name: &[u8], mode: u16) -> crate::Result<Ino>;
    fn unlink(&self, parent: Ino, name: &[u8]) -> crate::Result<()>;
    fn mkdir(&self, parent: Ino, name: &[u8], mode: u16) -> crate::Result<Ino>;
    fn rmdir(&self, parent: Ino, name: &[u8]) -> crate::Result<()>;
    fn rename(
        &self,
        old_parent: Ino,
        old_name: &[u8],
        new_parent: Ino,
        new_name: &[u8],
    ) -> crate::Result<()>;
    fn symlink(&self, parent: Ino, name: &[u8], target: &[u8]) -> crate::Result<Ino>;
    fn readlink(&self, ino: Ino) -> crate::Result<Vec<u8>>;
    fn chmod(&self, ino: Ino, mode: u16) -> crate::Result<()>;
    fn chown(&self, ino: Ino, uid: u32, gid: u32) -> crate::Result<()>;
    fn sync(&self) -> crate::Result<()>;
}
