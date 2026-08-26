# LinFS — Architecture

> 100% Rust. Windows. User-space Linux FS. chroot + terminal.

## 0. Principles

- **No kernel driver written by us.** We drive `winfsp.sys` (signed) via `winfsp-rs`; all FS logic is Rust userspace. Keeps 100% Rust ratio honest (ratio tool excludes `.sys`).
- **No Python/Java/Node product code.** UI glue is `egui` (Rust) or `axum` + thin `ui/` JS — same pattern as GSV `src/` + `ui/`.
- **Parsers are fallible.** Every superblock/B+tree/journal parser returns `Result<_, Corruption>` and is fuzzed. Never `unwrap` on guest data.
- **Journal before data.** Write path: `journal::Tx` → data blocks → commit → checkpoint. Matches `jbd2`.

---

## 1. Crate Map

```
linfs/                          # workspace root  S:\rust\LinFS
├── Cargo.toml                  # [workspace] resolver 2, members = [crates/*, linfs-cli, linfs-gui]
├── crates/
│   ├── linfs-core/             # traits + errors, no OS
│   │   └── src/{fs.rs, block.rs, inode.rs, error.rs}
│   ├── linfs-block/            # Windows block devices + image files
│   │   └── src/{win.rs, image.rs, partition/{mbr,gpt,lvm,luks,md}.rs}
│   ├── linfs-fs/               # FS drivers behind a common VFS
│   │   └── src/{vfs.rs, ext4/{super,extent,dir,htree,journal,alloc,inline}.rs,
│   │                    xfs/{ag,sb,bptree,log}.rs,
│   │                    btrfs/{super,chunk,btrfs_btree,compress}.rs,
│   │                    f2fs/{super,nat,sit,checkpoint}.rs}
│   ├── linfs-mount/            # WinFSP bridge + internal VFS server
│   │   └── src/{winfsp.rs, fallback.rs, options.rs}
│   ├── linfs-chroot/           # path translator + bind mounts + synthetic /proc /dev
│   │   └── src/{root.rs, bind.rs, proc.rs, dev.rs}
│   ├── linfs-terminal/         # ConPTY + busybox ELF loader + WSL bridge
│   │   └── src/{pty.rs, busybox.rs, wsl.rs, elf_loader.rs}
│   ├── linfs-gui/              # egui or axum+ui — file browser + editor + terminal widget
│   └── linfs-cli/              # linfs.exe CLI (attach/mount/chroot/exec/detach)
│   └── xtask/                  # cargo xtask {products,disk,sync,speed,rust}
├── ui/                         # thin HTML/CSS/JS if axum mode (no build step)
├── benches/                    # criterion: ls -R, 4K random read, journal commit
├── tests/                      # integration: loopback .img fixtures
└── data/                       # .gitignored runtime (mounts/history)
```

**Dependency floors:** `edition 2021`, `tokio 1`, `axum 0.7` (if server mode), `windows 0.58`, `winfsp 0.3`, `egui 0.28` or `tauri 1.6` (choose one at Band 2), `crc32fast`, `lz4_flex`, `zstd`, `argon2`, `sha2`, `clap`.

---

## 2. Data Flow

```
[PhysicalDrive / .img / .vhd]
        │  CreateFileW + IOCTLs
        ▼
   linfs-block::Device          ── raw Block (trait ReadAt + WriteAt + Len)
        │  partition scan
        ▼
   Partition { offset, len, ty } ──► LUKS/LVM/md resolver ──► LeafBlock
        │  fstype probe
        ▼
   linfs-fs::Vfs::open(LeafBlock) ──► ext4::Fs / xfs::Fs / btrfs::Fs / f2fs::Fs
        │  boxed as dyn FileSystem
        ▼
   linfs-mount::Mount  ─┬─► WinFSP drive M:\  (via winfsp-rs callbacks)
                        └─► linfs-chroot::Root (path translator)
                                   │
                                   ▼
                           linfs-terminal::Pty  (ConPTY + shell)
                                   │
                                   ▼
                           Monaco/hex editor (via gui)
```

---

## 3. Block Layer — `linfs-block`

### 3.1 Windows device

```rust
// crates/linfs-block/src/win.rs:12
pub struct WinDevice { handle: OwnedHandle, len: u64, sector: u32 }
impl Block for WinDevice {
    fn read_at(&self, off: u64, buf: &mut [u8]) -> io::Result<()>;
    fn write_at(&self, off: u64, buf: &[u8]) -> io::Result<()>;
    fn len(&self) -> u64;
    fn sector_size(&self) -> u32;
}
```

- Open `\\.\PhysicalDriveN` with `CreateFileW(..., GENERIC_READ|GENERIC_WRITE, FILE_SHARE_READ|WRITE, OPEN_EXISTING, FILE_FLAG_NO_BUFFERING|FILE_FLAG_OVERLAPPED)`. If `ACCESS_DENIED`, retry `GENERIC_READ` → ro.
- `DeviceIoControl(IOCTL_DISK_GET_DRIVE_GEOMETRY_EX)` → sector size; `IOCTL_DISK_GET_LENGTH_INFO` → len; `FSCTL_ALLOW_EXTENDED_DASD_IO` to allow partition reads when volume is locked.
- Alignment: all `ReadFile` buffers aligned to sector; wrapper handles unaligned sub-reads.
- Hotplug: `RegisterDeviceNotificationW` + `WM_DEVICECHANGE` polling thread (`linfs-block/src/hotplug.rs:1`) pushes `Event::Arrived/Removed`.

### 3.2 Image files

`ImageDevice` wraps `std::fs::File` with same `Block` trait. Supports `qcow2`/`vhd` via `linfs-block/src/image/qcow2.rs:1` (Rust parser, no `qemu-img`). `*.img` is just raw `File`.

### 3.3 Partition parsers

- `mbr.rs:1` — 512-byte MBR, 4 entries, EBR chain for extended. Validates `0x55AA`.
- `gpt.rs:1` — LBA 1 header, CRC32, backup LBA, entries array (128 × 128 B). Handles corrupt backup.
- `lvm.rs:1` — scan each partition for `LABELONE` at sector 1, parse PV header, VG metadata (text), LV mappings. Expose each LV as `Block` slicing the underlying device.
- `luks.rs:1` — LUKS2 JSON header at offset 0, `argon2id` KDF (Rust `argon2` crate), `aes-xts-plain64` via `aes`+`xts-mode` crates. Returns `LuksBlock` decrypting on read/write.
- `md.rs:1` — mdadm 1.2 superblock at 4 KiB from end, RAID level, chunk size. RAID1 = mirror read, RAID0 = stripe, RAID5 = ro with parity check.

All parsers are `no_std`-friendly and fuzzed (`fuzz/fuzz_targets/partition_*.rs`).

---

## 4. FS Drivers — `linfs-fs`

### 4.1 Common VFS trait (`linfs-core/src/fs.rs:1`)

```rust
pub trait FileSystem: Send {
    fn statfs(&self) -> FsStat;
    fn lookup(&self, parent: Ino, name: &[u8]) -> Result<Ino>;
    fn getattr(&self, ino: Ino) -> Result<Attr>;
    fn readdir(&self, ino: Ino) -> Result<Vec<Dirent>>;
    fn read(&self, ino: Ino, off: u64, buf: &mut [u8]) -> Result<usize>;
    fn write(&self, ino: Ino, off: u64, buf: &[u8]) -> Result<usize>;
    fn create(&self, parent: Ino, name: &[u8], mode: u16) -> Result<Ino>;
    fn unlink(&self, parent: Ino, name: &[u8]) -> Result<()>;
    // ... mkdir, rmdir, rename, symlink, readlink, link, chmod, chown, xattr
    fn sync(&self) -> Result<()>;
}
```

Every driver structs holds `Arc<dyn Block>` + `RwLock<Cache>` (block cache 4 KiB) + `Journal`.

### 4.2 ext4 — P0 driver (`linfs-fs/src/ext4/`)

Key modules:

| File | Responsibility |
|------|---------------|
| `super.rs:1` | Superblock `0x400+` (1024 B), `s_feature_*`, `s_blocks_per_group`, `s_inodes_per_group`, `s_first_data_block`, `64bit` high words, checksum (`crc32c`). Validates `s_magic==0xEF53`. |
| `group.rs:1` | Block group descriptors (32/64 B), `flex_bg` merging, `meta_bg`. |
| `extent.rs:1` | Extent tree (`ext4_extent_header` → leaf `ee_block+ee_len+ee_start`). Handles `unwritten` flag, `inline_data`. |
| `dir.rs:1` | `ext4_dir_entry_2` linear + `htree` (`dx_root` → `dx_node` hash `half_md4`). |
| `journal.rs:1` | `jbd2` descriptor/commit/revoke. Replay on mount if `EXT4_FEATURE_INCOMPAT_RECOVER`. Tx: `Tx { buffers: Vec<Buf> }` → journal blocks → commit block with checksum. |
| `alloc.rs:1` | Buddy bitmap alloc (block `bg_block_bitmap`, inode `bg_inode_bitmap`). |
| `xattr.rs:1` | `ext4_xattr_entry` inline vs block. |

Journal replay algorithm (`journal.rs:120`): scan journal superblock → find `s_sequence` tail/head → replay descriptor+data blocks → apply to FS blocks → clear `needs_recovery`.

### 4.2.1 ext4 write path

```
create("/etc/hostname") → alloc inode (bitmap) → alloc blocks (bitmap) → write dir entry (htree rebalance if needed) → journal Tx { bitmap blocks, dir block, inode table block }
```

All mutations go through `Journal::transact(|tx| ...)` which pins blocks, writes to journal, then checkpoints.

### 4.3 xfs / btrfs / f2fs

- `xfs`: AGs (`xfs_sb` per AG), B+tree for `inode`/`free`/`rmap`, log at `sb_logstart`. Log replay similar to jbd2 but `xfs_log` format.
- `btrfs`: `superblock` at `0x10000`, `chunk tree` → logical→physical, `fs tree` (B-tree with `btrfs_key`), `compress` (`zlib`/`lzo`/`zstd` decompress on read, compress on write if `compress` mount opt).
- `f2fs`: `superblock` @ 0, two checkpoints (pick valid CRC), `NAT` (node address table), `SIT` (segment info). Checkpoint replay picks newer `checkpoint_ver`.

Each driver has `probe(block) -> bool` reading superblock magic: `ext4 0xEF53`, `xfs "XFSB"`, `btrfs "_BHRfS_M"`, `f2fs 0xF2F52010`.

---

## 5. Mount — `linfs-mount`

### WinFSP bridge (`winfsp.rs:1`)

Implements `winfsp::Filesystem` trait:

```rust
impl winfsp::Filesystem for LinfsFs {
    fn get_volume_info(&self) -> VolumeInfo;
    fn open(&self, path: &str, ...) -> Result<FileContext>;
    fn read(&self, ctx: &FileContext, off: u64, buf: &mut [u8]) -> Result<usize>;
    fn write(&self, ctx: &FileContext, off: u64, buf: &[u8]) -> Result<usize>;
    // getattr, setattr, readdir, create, cleanup, flush, set_volume_label
}
```

- Path mapping: Windows `M:\etc\hostname` → VFS `"/etc/hostname"` (UTF-16 → WTF-8 → bytes, preserve case).
- Caching: WinFSP `FileInfo` cache 1 s; our block cache 16 MiB LRU (`lru` crate).
- `FLT` (filter) — deny Windows `:$DATA` streams; map `FILE_ATTRIBUTE_REPARSE_POINT` for symlinks (`IO_REPARSE_TAG_SYMLINK`).

Fallback (`fallback.rs:1`) exposes same `FileSystem` trait but over `axum` `GET /api/fs/*` for the internal browser — no WinFSP needed.

---

## 6. chroot — `linfs-chroot`

```rust
// crates/linfs-chroot/src/root.rs:1
pub struct Root { fs: Arc<dyn FileSystem>, inner: PathBuf /* mount point on host, not used */ }
impl Root {
    pub fn resolve(&self, cwd: &str, path: &str) -> Result<Resolved>; // chroot translation
    pub fn bind(&self, host: &Path, guest: &str) -> Result<()>;
}
```

- `resolve(cwd, "/etc/hostname")` → `"/etc/hostname"` inside FS (leading `/` anchored to `Root`). `resolve(cwd, "../../etc")` with `cwd="/"` → clamps to `"/etc"` (no escape).
- `bind`: inserts into `BTreeMap<GuestPath, HostPath>`; `resolve` checks longest prefix match → delegates to `std::fs` for host paths.
- Synthetic: `proc.rs:1` generates `/proc/mounts` (text), `/proc/self/mountinfo`; `dev.rs:1` synthesizes `null/zero/random/urandom` as in-memory inodes with `read` handlers.

chroot is **not** a Windows `chroot(2)` syscall — it's a logical root for `linfs-fs` and `linfs-terminal`. Multiple `Root`s coexist.

---

## 7. Terminal — `linfs-terminal`

### 7.1 ConPTY (`pty.rs:1`)

```rust
pub struct Pty { hpc: HPCON, stdin: Pipe, stdout: Pipe }
impl Pty {
    pub fn spawn(shell: &str, cols: u16, rows: u16) -> Result<Self>;
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()>;
}
```

Wraps `CreatePseudoConsole` + `CreateProcessW` (`EXTENDED_STARTUPINFO_PRESENT`). Input from GUI → `WriteFile(stdin)`; output → `ReadFile(stdout)` → ANSI parser → GUI.

### 7.2 Tier 1: busybox shim (`busybox.rs:1`, `elf_loader.rs:1`)

- Ship `busybox-x86_64` (≈ 1 MiB static, built with `musl`, embedded via `include_bytes!("../assets/busybox")`).
- `elf_loader.rs:1` parses ELF64 (`0x7F 'E' 'L' 'F'`), maps `PT_LOAD`, resolves `PT_INTERP` → `musl` loader, and shims ~40 syscalls: `open/read/write/close/lseek/stat/fstat/mkdir/unlink/rename/chdir/getcwd/fork/execve/wait4` — enough for `sh` pipeline. Each syscall translates via `Root::resolve` to `FileSystem` ops.
- Limitation: no `mmap` of guest `.so` — only static busybox in MVP; dynamic ELF stretch.

### 7.3 Tier 2: WSL bridge (`wsl.rs:1`)

```rust
pub fn wsl_available() -> bool { Command::new("wsl").arg("--status").status().is_ok() }
pub fn wsl_chroot(mount: &str, cmd: &str) -> Result<Pty> {
    // wsl --mount --bare \\.\PhysicalDrive2 --partition 2 --type ext4  (if direct)
    // or wsl bash -c "mount -t drvfs M: /mnt/linfs && chroot /mnt/linfs /bin/bash"
}
```

If `WSL_AVAILABLE`, `linfs-terminal` spawns `wsl.exe bash -c "chroot /mnt/linfs/<id> /bin/bash"` where `/mnt/linfs/<id>` is the WinFSP drive mounted as `drvfs` inside WSL (WSL auto-mounts `M:` as `/mnt/m`). Guest's real `/bin/bash` runs under real Linux kernel — 100% compat.

Selection: `Terminal::spawn(root, mode: Auto|Busybox|Wsl)` — `Auto` prefers WSL if present, else busybox.

---

## 8. GUI — `linfs-gui`

Choice at Band 2:

- **Option A (recommended, reuses GSV pattern):** `axum` server `127.0.0.1:9998` + `ui/` vanilla JS + `xterm.js` for terminal. No Tauri. Thin, 100% Rust backend. Reuses GSV's `gsv-server` playbook (manifest/feed/speed chops not needed, but `preview`/`terminal` boxes reuse).
- **Option B:** `egui` native window (`eframe`) — single `.exe`, no browser, `egui_term` for terminal. Heavier but offline-first.

Both call `linfs-core` directly (same process). GUI threads: UI (main), block I/O (tokio), WinFSP callback (dedicated thread, must not block).

State: `AppState { devices: Vec<Device>, mounts: HashMap<Id, Mount>, roots: HashMap<Id, Root> }` behind `Arc<RwLock>`.

---

## 9. CLI — `linfs-cli`

```
linfs list                          # enumerate devices + partitions
linfs attach \\.\PhysicalDrive2      # or path\to\disk.img
linfs mount 2:2 --rw --drive M      # partition 2 on disk 2
linfs ls   M:/etc
linfs cat  M:/etc/hostname
linfs edit M:/etc/hostname          # $EDITOR
linfs chroot M: -- bind C:\share /mnt/host -- bash
linfs exec M: -- /bin/bash -lc "apt list --installed | head"
linfs umount M:
linfs detach 2
linfs fsck 2:2
```

CLI and GUI share `linfs-core` — GUI is just a frontend.

---

## 10. Testing Strategy

| Level | Tool | Example |
|-------|------|---------|
| Unit | `cargo test` | `ext4::super::test_parse_superblock`, `gpt::test_crc` |
| Property | `proptest` | `rename(a,b) then rename(b,a)` is idempotent; `journal replay` twice == once |
| Fuzz | `cargo fuzz` | `fuzz_targets/ext4_dir`, `fuzz_targets/btrfs_btree` (OSS-Fuzz style) |
| Integration | loopback `.img` fixtures (checked into `tests/fixtures/*.img` — 8 MiB each, generated by `scripts/mkfixtures.sh` on Linux) | `mount_ext4_rw_edit_sync` — open fixture, write, sync, reopen and read back; compare with `debugfs` dump |
| WinFSP | manual + CI with `winfsp` installed | `cargo test --features winfsp` mounts to `T:` and runs `TestDrive` |
| Perf | `criterion` | `benches/ls_r.rs`, `benches/journal_commit.rs` |

CI: Windows runner (`windows-2022`, `x86_64-pc-windows-gnu`), installs WinFSP, runs `cargo test` + `cargo clippy -- -D warnings`.

---

## 11. Security & Safety

- Elevation: `UAC` prompt only on `WinDevice::open_rw`; image files never elevate.
- `needs_recovery` gate: refuse `rw` if journal replay failed → force `ro` + `fsck` suggestion.
- No write to `Disk 0` without explicit `--force` (protect Windows system disk).
- Path traversal: `Root::resolve` clamps `..` at `/`; WinFSP `open` rejects `..` components after translation.
- Fuzz + `cargo audit` + `cargo deny` in CI.

---

## 12. Trade-offs

| Decision | Chosen | Alternative | Why |
|----------|--------|-------------|-----|
| WinFSP vs raw `\\.\PhysicalDrive` passthrough to WSL | WinFSP primary, WSL optional | Pure WSL `wsl --mount` | WSL requires admin + Hyper-V; WinFSP works on stock Windows, no kernel. WSL is Tier 2 compat layer, not sole path. |
| ext4 driver clean-room vs `ext4-rs` crate | Fork `ext4-rs`/`ext4-view` (MIT) then harden | Bind `libext2fs` (C) | Keep 100% Rust ratio; C FFI would break canon. |
| Terminal busybox shim vs full qemu-user | busybox shim MVP | `qemu-x86_64` static | qemu is C, large, GPL; shim is Rust, small, covers 80% edits. |
| `egui` vs `axum+ui` | `axum+ui` (GSV pattern) | `tauri` | Reuses GSV `ui/` + preview/terminal boxes; `tauri` pulls Node. `egui` is fallback if browser banned. |
| `tokio` sync vs `std::sync` for FS cache | `parking_lot::RwLock` (sync) | `tokio::RwLock` | FS callbacks are sync (WinFSP thread); async only at block I/O boundary. |

---

## 13. What We Revisit Later

- `btrfs` RAID56 rw — needs full extent allocator, defer.
- `fscrypt` / `ext4` encryption — needs kernel keyring integration.
- `qcow2` write — start ro, add rw after journal safety proven.
- `aarch64` guest emulation — needs `qemu-user` or Rust `rv64` dynarec; stretch.
