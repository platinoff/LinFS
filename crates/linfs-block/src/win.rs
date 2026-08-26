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
    pub fn open(_path: &str) -> linfs_core::Result<Arc<Self>> {
        // TODO: implement full open + IOCTL_DISK_GET_LENGTH_INFO + GET_DRIVE_GEOMETRY_EX
        Err(linfs_core::Error::Unsupported(
            "WinDevice::open not yet implemented".into(),
        ))
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

/// Enumerate \\.\PhysicalDriveN that exist (0..16 probe).
pub fn enumerate() -> Vec<String> {
    #[cfg(windows)]
    {
        (0..16)
            .map(|n| format!(r"\\.\PhysicalDrive{n}"))
            .filter(|p| std::path::Path::new(p).exists())
            .collect()
    }
    #[cfg(not(windows))]
    {
        vec![]
    }
}
