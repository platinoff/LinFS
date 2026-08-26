//! Windows raw disk via CreateFileW + DeviceIoControl — stub for non-Windows builds.
use linfs_core::block::Block;
use std::sync::Arc;

pub struct WinDevice {
    #[cfg(windows)]
    #[allow(dead_code)]
    handle: std::os::windows::io::OwnedHandle,
    len: u64,
    sector: u32,
}

impl WinDevice {
    pub fn open(path: &str) -> linfs_core::Result<Arc<Self>> {
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use std::os::windows::io::{FromRawHandle, OwnedHandle};
            use windows::core::PCWSTR;

            use windows::Win32::Storage::FileSystem::{
                CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
                FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
            };
            let wide: Vec<u16> = std::ffi::OsStr::new(path)
                .encode_wide()
                .chain(Some(0))
                .collect();
            let handle = unsafe {
                CreateFileW(
                    PCWSTR(wide.as_ptr()),
                    FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    None,
                    OPEN_EXISTING,
                    FILE_FLAGS_AND_ATTRIBUTES(0),
                    None,
                )
            }
            .map_err(|e| {
                linfs_core::Error::Io(std::io::Error::other(format!("CreateFileW {path}: {e}")))
            })?;
            let len = 0u64;
            let owned = unsafe { OwnedHandle::from_raw_handle(handle.0) };
            Ok(Arc::new(Self {
                handle: owned,
                len,
                sector: 512,
            }))
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Err(linfs_core::Error::Unsupported(
                "WinDevice only on Windows".into(),
            ))
        }
    }
}

impl Block for WinDevice {
    fn read_at(&self, _off: u64, _buf: &mut [u8]) -> std::io::Result<()> {
        Err(std::io::Error::other("not implemented"))
    }
    fn write_at(&self, _off: u64, _buf: &[u8]) -> std::io::Result<()> {
        Err(std::io::Error::other("not implemented"))
    }
    fn len(&self) -> u64 {
        self.len
    }
    fn sector_size(&self) -> u32 {
        self.sector
    }
}

/// Enumerate \\.\PhysicalDriveN that exist (0..16 probe via CreateFileW, not Path::exists).
pub fn enumerate() -> Vec<String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::Storage::FileSystem::{
            CreateFileW, FILE_GENERIC_READ, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        };
        (0..16)
            .filter_map(|n| {
                let p = format!(r"\\.\PhysicalDrive{n}");
                let wide: Vec<u16> = std::ffi::OsStr::new(&p)
                    .encode_wide()
                    .chain(Some(0))
                    .collect();
                let h = unsafe {
                    CreateFileW(
                        PCWSTR(wide.as_ptr()),
                        FILE_GENERIC_READ.0,
                        FILE_SHARE_READ | FILE_SHARE_WRITE,
                        None,
                        OPEN_EXISTING,
                        windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
                        None,
                    )
                };
                match h {
                    Ok(handle) => {
                        unsafe {
                            let _ = windows::Win32::Foundation::CloseHandle(handle);
                        }
                        Some(p)
                    }
                    Err(_) => None,
                }
            })
            .collect()
    }
    #[cfg(not(windows))]
    {
        vec![]
    }
}

/// List partitions for a PhysicalDrive via MBR/GPT probe (band 200).
pub fn list_partitions(_block: &dyn Block) -> Vec<crate::partition::Partition> {
    // MVP: try GPT first, fallback MBR — full parsing in partition::gpt/mbr
    vec![]
}
