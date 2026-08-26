use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "linfs",
    about = "LinFS — mount and mutate Linux filesystems on Windows"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Enumerate block devices and partitions
    List,
    /// Attach a raw image file
    Attach { path: std::path::PathBuf },
    /// Mount a partition (e.g. 2:2) to a drive letter
    Mount {
        spec: String,
        #[arg(long)]
        drive: Option<String>,
    },
    /// List files at a path
    Ls { path: String },
    /// chroot into a mount and run a command
    Chroot {
        root: String,
        #[arg(last = true)]
        cmd: Vec<String>,
    },
    /// Check filesystem
    Fsck { spec: String },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::List => {
            println!("Probing block devices...");
            for dev in linfs_block::win::enumerate() {
                println!("{dev}");
            }
            // also probe image dir
            println!("(image attach: linfs attach <path>)");
        }
        Cmd::Attach { path } => {
            let _dev = linfs_block::image::ImageDevice::open(&path)?;
            println!("attached {path:?}");
        }
        Cmd::Mount { spec, drive } => {
            println!("mount {spec} -> {drive:?} (not yet implemented, band 203)");
        }
        Cmd::Ls { path } => {
            println!("ls {path} (not yet implemented, band 203)");
        }
        Cmd::Chroot { root, cmd } => {
            // Try open as image file; if `root` is a path to .img, open it and chroot
            let fs: std::sync::Arc<dyn linfs_core::fs::FileSystem> = {
                // Dummy FS for path-translation demo if file not found
                struct DummyFs;
                impl linfs_core::fs::FileSystem for DummyFs {
                    fn statfs(&self) -> linfs_core::fs::FsStat {
                        linfs_core::fs::FsStat {
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
                    fn getattr(&self, _: u64) -> linfs_core::Result<linfs_core::fs::Attr> {
                        Err(linfs_core::Error::NotFound("dummy".into()))
                    }
                    fn readdir(&self, _: u64) -> linfs_core::Result<Vec<linfs_core::fs::Dirent>> {
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
                // Try real ext4 open if path exists as image
                let p = std::path::Path::new(&root);
                if p.is_file() {
                    // For MVP, try open as ext4 image; fallback to dummy
                    match linfs_block::image::ImageDevice::open(p) {
                        Ok(dev) => {
                            let arc: std::sync::Arc<dyn linfs_core::block::Block> =
                                std::sync::Arc::new(dev);
                            match linfs_fs::ext4::Fs::open(arc.clone()) {
                                Ok(fs) => {
                                    // Wrap ext4 Fs as FileSystem via adapter (readdir -> Dirent)
                                    struct Ext4Adapter(linfs_fs::ext4::Fs);
                                    impl linfs_core::fs::FileSystem for Ext4Adapter {
                                        fn statfs(&self) -> linfs_core::fs::FsStat {
                                            linfs_core::fs::FsStat {
                                                blocks: 0,
                                                bfree: 0,
                                                bsize: self.0.block_size(),
                                                files: 0,
                                                ffree: 0,
                                            }
                                        }
                                        fn lookup(
                                            &self,
                                            p: u64,
                                            n: &[u8],
                                        ) -> linfs_core::Result<u64>
                                        {
                                            Ok(self.0.lookup(p as u32, n)? as u64)
                                        }
                                        fn getattr(
                                            &self,
                                            ino: u64,
                                        ) -> linfs_core::Result<linfs_core::fs::Attr>
                                        {
                                            let i = self.0.getattr(ino as u32)?;
                                            Ok(linfs_core::fs::Attr {
                                                ino,
                                                mode: i.mode,
                                                uid: i.uid as u32,
                                                gid: i.gid as u32,
                                                size: i.size(),
                                                nlink: i.links_count as u32,
                                                mtime: i.mtime as i64,
                                                is_dir: i.is_dir(),
                                                is_symlink: false,
                                            })
                                        }
                                        fn readdir(
                                            &self,
                                            ino: u64,
                                        ) -> linfs_core::Result<Vec<linfs_core::fs::Dirent>>
                                        {
                                            let v = self.0.readdir(ino as u32)?;
                                            Ok(v.into_iter()
                                                .map(|e| linfs_core::fs::Dirent {
                                                    ino: e.inode as u64,
                                                    name: e.name,
                                                    is_dir: e.file_type == 2,
                                                })
                                                .collect())
                                        }
                                        fn read(
                                            &self,
                                            _: u64,
                                            _: u64,
                                            _: &mut [u8],
                                        ) -> linfs_core::Result<usize>
                                        {
                                            Ok(0)
                                        }
                                        fn write(
                                            &self,
                                            _: u64,
                                            _: u64,
                                            _: &[u8],
                                        ) -> linfs_core::Result<usize>
                                        {
                                            Ok(0)
                                        }
                                        fn create(
                                            &self,
                                            _: u64,
                                            _: &[u8],
                                            _: u16,
                                        ) -> linfs_core::Result<u64>
                                        {
                                            Ok(1)
                                        }
                                        fn unlink(
                                            &self,
                                            _: u64,
                                            _: &[u8],
                                        ) -> linfs_core::Result<()>
                                        {
                                            Ok(())
                                        }
                                        fn mkdir(
                                            &self,
                                            _: u64,
                                            _: &[u8],
                                            _: u16,
                                        ) -> linfs_core::Result<u64>
                                        {
                                            Ok(1)
                                        }
                                        fn rmdir(
                                            &self,
                                            _: u64,
                                            _: &[u8],
                                        ) -> linfs_core::Result<()>
                                        {
                                            Ok(())
                                        }
                                        fn rename(
                                            &self,
                                            _: u64,
                                            _: &[u8],
                                            _: u64,
                                            _: &[u8],
                                        ) -> linfs_core::Result<()>
                                        {
                                            Ok(())
                                        }
                                        fn symlink(
                                            &self,
                                            _: u64,
                                            _: &[u8],
                                            _: &[u8],
                                        ) -> linfs_core::Result<u64>
                                        {
                                            Ok(1)
                                        }
                                        fn readlink(&self, _: u64) -> linfs_core::Result<Vec<u8>> {
                                            Ok(vec![])
                                        }
                                        fn chmod(&self, _: u64, _: u16) -> linfs_core::Result<()> {
                                            Ok(())
                                        }
                                        fn chown(
                                            &self,
                                            _: u64,
                                            _: u32,
                                            _: u32,
                                        ) -> linfs_core::Result<()>
                                        {
                                            Ok(())
                                        }
                                        fn sync(&self) -> linfs_core::Result<()> {
                                            Ok(())
                                        }
                                    }
                                    std::sync::Arc::new(Ext4Adapter(fs))
                                        as std::sync::Arc<dyn linfs_core::fs::FileSystem>
                                }
                                Err(_) => std::sync::Arc::new(DummyFs)
                                    as std::sync::Arc<dyn linfs_core::fs::FileSystem>,
                            }
                        }
                        Err(_) => std::sync::Arc::new(DummyFs)
                            as std::sync::Arc<dyn linfs_core::fs::FileSystem>,
                    }
                } else {
                    std::sync::Arc::new(DummyFs) as std::sync::Arc<dyn linfs_core::fs::FileSystem>
                }
            };
            let chroot = linfs_chroot::root::Root::new(fs);
            // Demo bind: if root is image, also bind host share example
            if cmd.is_empty() {
                println!("chroot {root} — enter (cwd /)");
                // Interactive: spawn shell via Pty with chroot cwd /
                let shell = if cfg!(windows) { "cmd.exe" } else { "sh" };
                println!("spawning {shell} in chroot / (binds: none)");
                match linfs_terminal::pty::Pty::spawn(shell, 80, 24) {
                    Ok(mut pty) => {
                        let out = pty
                            .read_timeout(std::time::Duration::from_secs(1))
                            .unwrap_or_default();
                        println!("{out}");
                    }
                    Err(e) => eprintln!("pty spawn failed: {e}"),
                }
            } else {
                let joined = cmd.join(" ");
                println!("chroot {root} -- {joined}");
                // Resolve first arg as path inside chroot for demo
                if let Some(first) = cmd.first() {
                    // If cmd is like "cat /etc/hostname", resolve /etc/hostname
                    let path = if first == "cat" && cmd.len() > 1 {
                        cmd[1].as_str()
                    } else {
                        first.as_str()
                    };
                    match chroot.resolve("/", path) {
                        Ok(res) => {
                            println!("resolve {} -> {:?} (is_host={})", path, res, res.is_host())
                        }
                        Err(e) => eprintln!("resolve failed: {e}"),
                    }
                }
                // Execute via Pty
                let shell_cmd = joined;
                match linfs_terminal::pty::Pty::spawn(&shell_cmd, 80, 24) {
                    Ok(mut pty) => {
                        let out = pty
                            .read_timeout(std::time::Duration::from_secs(2))
                            .unwrap_or_default();
                        println!("{out}");
                    }
                    Err(e) => eprintln!("pty spawn failed: {e}"),
                }
            }
        }
        Cmd::Fsck { spec } => {
            println!("fsck {spec} (not yet implemented, band 210)");
        }
    }
    Ok(())
}
