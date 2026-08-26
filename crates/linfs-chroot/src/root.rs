use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    Guest(String),
    Host(PathBuf),
}

impl Resolved {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Guest(s) => s.as_str(),
            Self::Host(p) => p.to_str().unwrap_or(""),
        }
    }
    pub fn is_host(&self) -> bool {
        matches!(self, Self::Host(_))
    }
    pub fn is_guest(&self) -> bool {
        matches!(self, Self::Guest(_))
    }
}

/// Chroot path translator — clamps `..` at `/`, resolves host binds.
pub struct Root {
    _fs: std::sync::Arc<dyn linfs_core::fs::FileSystem>,
    binds: std::sync::RwLock<BTreeMap<String, PathBuf>>,
}

impl Root {
    pub fn new(fs: std::sync::Arc<dyn linfs_core::fs::FileSystem>) -> Self {
        Self {
            _fs: fs,
            binds: std::sync::RwLock::new(BTreeMap::new()),
        }
    }

    /// Bind a Windows host path at a guest absolute path (e.g., `C:\share` → `/mnt/host`).
    pub fn bind(&self, host: &Path, guest: &str) -> linfs_core::Result<()> {
        if !guest.starts_with('/') {
            return Err(linfs_core::Error::Corruption(format!(
                "bind guest must be absolute: {guest}"
            )));
        }
        let guest = normalize_guest(guest);
        self.binds
            .write()
            .unwrap()
            .insert(guest, host.to_path_buf());
        Ok(())
    }

    /// Resolve `path` (absolute or relative to `cwd`) inside chroot.
    /// Returns `Guest("/etc/hosts")` or `Host("C:\\share\\foo")` if under a bind.
    pub fn resolve(&self, cwd: &str, path: &str) -> linfs_core::Result<Resolved> {
        let combined = if path.starts_with('/') {
            path.to_string()
        } else {
            let cwd = cwd.trim_end_matches('/');
            if cwd.is_empty() || cwd == "/" {
                format!("/{}", path)
            } else {
                format!("{}/{}", cwd, path)
            }
        };
        let normalized = normalize_guest(&combined);
        // Longest prefix bind match
        let binds = self.binds.read().unwrap();
        let mut best: Option<(&String, &PathBuf)> = None;
        for (guest, host) in binds.iter() {
            if normalized == *guest || normalized.starts_with(&format!("{guest}/")) {
                match best {
                    None => best = Some((guest, host)),
                    Some((prev, _)) if guest.len() > prev.len() => best = Some((guest, host)),
                    _ => {}
                }
            }
        }
        if let Some((guest_prefix, host_base)) = best {
            let suffix = &normalized[guest_prefix.len()..];
            let suffix = suffix.trim_start_matches('/');
            let host_path = if suffix.is_empty() {
                host_base.clone()
            } else {
                host_base.join(suffix.replace('/', "\\"))
            };
            return Ok(Resolved::Host(host_path));
        }
        Ok(Resolved::Guest(normalized))
    }
}

fn normalize_guest(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for comp in path.split('/') {
        match comp {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(comp),
        }
    }
    if parts.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", parts.join("/"))
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
            Err(linfs_core::Error::NotFound("dummy".into()))
        }
        fn getattr(&self, _: u64) -> linfs_core::Result<Attr> {
            Err(linfs_core::Error::NotFound("dummy".into()))
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

    fn dummy_root() -> Root {
        Root::new(std::sync::Arc::new(DummyFs))
    }

    #[test]
    fn chroot_clamps_dotdot() {
        let r = dummy_root();
        assert_eq!(
            r.resolve("/", "../../etc").unwrap(),
            Resolved::Guest("/etc".into())
        );
        assert_eq!(
            r.resolve("/etc", "../var").unwrap(),
            Resolved::Guest("/var".into())
        );
        assert_eq!(
            r.resolve("/a/b", "../../..").unwrap(),
            Resolved::Guest("/".into())
        );
    }

    #[test]
    fn chroot_normalizes() {
        let r = dummy_root();
        assert_eq!(
            r.resolve("/etc", "./hosts").unwrap(),
            Resolved::Guest("/etc/hosts".into())
        );
        assert_eq!(
            r.resolve("/", "/etc//hosts/").unwrap(),
            Resolved::Guest("/etc/hosts".into())
        );
    }

    #[test]
    fn chroot_bind_host() {
        let r = dummy_root();
        r.bind(Path::new("C:\\tmp"), "/mnt/host").unwrap();
        assert_eq!(
            r.resolve("/", "/mnt/host/foo").unwrap(),
            Resolved::Host(PathBuf::from("C:\\tmp\\foo"))
        );
        assert!(r.resolve("/", "/mnt/host/foo").unwrap().is_host());
        // non-bound stays guest
        assert!(r.resolve("/", "/etc/hosts").unwrap().is_guest());
    }

    #[test]
    fn chroot_bind_prefix_longest() {
        let r = dummy_root();
        r.bind(Path::new("C:\\a"), "/mnt").unwrap();
        r.bind(Path::new("D:\\b"), "/mnt/host").unwrap();
        // longest prefix wins
        assert_eq!(
            r.resolve("/", "/mnt/host/x").unwrap(),
            Resolved::Host(PathBuf::from("D:\\b\\x"))
        );
        assert_eq!(
            r.resolve("/", "/mnt/other").unwrap(),
            Resolved::Host(PathBuf::from("C:\\a\\other"))
        );
    }
}
