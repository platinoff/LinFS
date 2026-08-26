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
            println!("chroot {root} -- {cmd:?} (not yet implemented, band 204)");
        }
        Cmd::Fsck { spec } => {
            println!("fsck {spec} (not yet implemented, band 210)");
        }
    }
    Ok(())
}
