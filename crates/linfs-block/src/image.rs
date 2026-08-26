use linfs_core::block::Block;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::Mutex;

pub struct ImageDevice {
    file: Mutex<File>,
    len: u64,
}

impl ImageDevice {
    pub fn open(path: impl AsRef<std::path::Path>) -> linfs_core::Result<Self> {
        let file = File::options()
            .read(true)
            .write(true)
            .open(path.as_ref())
            .map_err(linfs_core::Error::Io)?;
        let len = file.metadata().map_err(linfs_core::Error::Io)?.len();
        Ok(Self {
            file: Mutex::new(file),
            len,
        })
    }
}

impl Block for ImageDevice {
    fn read_at(&self, off: u64, buf: &mut [u8]) -> std::io::Result<()> {
        let mut f = self.file.lock().unwrap();
        f.seek(SeekFrom::Start(off))?;
        f.read_exact(buf)
    }
    fn write_at(&self, off: u64, buf: &[u8]) -> std::io::Result<()> {
        let mut f = self.file.lock().unwrap();
        f.seek(SeekFrom::Start(off))?;
        f.write_all(buf)
    }
    fn len(&self) -> u64 {
        self.len
    }
}
