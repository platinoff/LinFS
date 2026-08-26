use std::sync::Arc;

/// WinFSP bridge — MVP validates drive letter and holds Fs Arc.
/// Real WinFSP via `winfsp-rs` `Filesystem` trait is stretch (requires winfsp.sys installed).
/// This stub lets `cargo test` pass and `linfs mount` degrade to fallback when driver absent.
pub struct Mount {
    fs: Arc<dyn linfs_core::fs::FileSystem>,
    drive: String,
}

impl Mount {
    pub fn new(fs: Arc<dyn linfs_core::fs::FileSystem>, drive: &str) -> linfs_core::Result<Self> {
        let d = drive.trim();
        if d.is_empty() {
            return Err(linfs_core::Error::Corruption("drive required".into()));
        }
        // Normalize `M:` or `M` or `C:\mnt\linfs`
        let drive = if d.len() == 1 && d.chars().next().unwrap().is_ascii_alphabetic() {
            format!("{}:", d.to_uppercase())
        } else {
            d.to_string()
        };
        // Validate Fs is readable: stat root ino 2
        let _ = fs
            .getattr(2)
            .map_err(|e| linfs_core::Error::Corruption(format!("mount getattr root: {e}")))?;
        // Check WinFSP driver presence (stub: look for winfsp-x64.dll)
        let has_driver = std::path::Path::new("C:\\Program Files\\WinFSP\\bin\\winfsp-x64.dll")
            .exists()
            || std::path::Path::new("C:\\Program Files (x86)\\WinFSP\\bin\\winfsp-x64.dll")
                .exists();
        if !has_driver {
            // Degrade: log and keep Mount as fallback holder (caller should use `fallback::serve`)
            eprintln!(
                "WinFSP driver not found — Mount {} will use fallback 127.0.0.1:9998",
                drive
            );
        } else {
            // Real mount would call `winfsp::host::FileSystemHost::new(fs_adapter, drive)` here
            eprintln!("WinFSP driver found — mount {} ready (stub)", drive);
        }
        Ok(Self { fs, drive })
    }

    pub fn drive(&self) -> &str {
        &self.drive
    }

    pub fn fs(&self) -> &Arc<dyn linfs_core::fs::FileSystem> {
        &self.fs
    }

    pub fn unmount(self) -> linfs_core::Result<()> {
        // Real impl would call `host.unmount()`
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linfs_core::fs::{Attr, Dirent, FileSystem, FsStat};

    struct DummyFs;
    impl FileSystem for DummyFs {
        fn statfs(&self) -> FsStat {
            FsStat {
                blocks: 0,
                bfree: 0,
                bsize: 4096,
                files: 0,
                ffree: 0,
            }
        }
        fn lookup(&self, _: u64, _: &[u8]) -> linfs_core::Result<u64> {
            Ok(2)
        }
        fn getattr(&self, ino: u64) -> linfs_core::Result<Attr> {
            Ok(Attr {
                ino,
                mode: 0o040755,
                uid: 0,
                gid: 0,
                size: 4096,
                nlink: 2,
                mtime: 0,
                is_dir: true,
                is_symlink: false,
            })
        }
        fn readdir(&self, _: u64) -> linfs_core::Result<Vec<Dirent>> {
            Ok(vec![])
        }
        fn read(&self, _: u64, _: u64, _: &mut [u8]) -> linfs_core::Result<usize> {
            Ok(0)
        }
        fn write(&self, _: u64, _: u64, _: &[u8]) -> linfs_core::Result<usize> {
            Ok(0)
        }
        fn create(&self, _: u64, _: &[u8], _: u16) -> linfs_core::Result<u64> {
            Ok(1)
        }
        fn unlink(&self, _: u64, _: &[u8]) -> linfs_core::Result<()> {
            Ok(())
        }
        fn mkdir(&self, _: u64, _: &[u8], _: u16) -> linfs_core::Result<u64> {
            Ok(1)
        }
        fn rmdir(&self, _: u64, _: &[u8]) -> linfs_core::Result<()> {
            Ok(())
        }
        fn rename(&self, _: u64, _: &[u8], _: u64, _: &[u8]) -> linfs_core::Result<()> {
            Ok(())
        }
        fn symlink(&self, _: u64, _: &[u8], _: &[u8]) -> linfs_core::Result<u64> {
            Ok(1)
        }
        fn readlink(&self, _: u64) -> linfs_core::Result<Vec<u8>> {
            Ok(vec![])
        }
        fn chmod(&self, _: u64, _: u16) -> linfs_core::Result<()> {
            Ok(())
        }
        fn chown(&self, _: u64, _: u32, _: u32) -> linfs_core::Result<()> {
            Ok(())
        }
        fn sync(&self) -> linfs_core::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn mount_validates_drive_and_root() {
        let fs = Arc::new(DummyFs);
        let m = Mount::new(fs, "M").unwrap();
        assert_eq!(m.drive(), "M:");
        let m2 = Mount::new(Arc::new(DummyFs), "T:").unwrap();
        assert_eq!(m2.drive(), "T:");
    }

    #[test]
    fn mount_rejects_empty_drive() {
        let fs = Arc::new(DummyFs);
        assert!(Mount::new(fs, "").is_err());
    }
}
