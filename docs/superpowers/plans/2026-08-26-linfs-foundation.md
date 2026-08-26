# LinFS Implementation Plan — Bands 200–204 (Foundation → MVP)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship LinFS MVP that mounts `ext4` read/write from any Windows block device or `.img`, browses/edits files via WinFSP or internal GUI, and provides `chroot` + Linux terminal to mutate the guest FS.

**Architecture:** Workspace `S:\rust\LinFS` with crates `linfs-core`/`linfs-block`/`linfs-fs`/`linfs-mount`/`linfs-chroot`/`linfs-terminal`/`linfs-gui`/`linfs-cli`; block layer via `CreateFileW`+`IOCTL`, FS drivers behind `FileSystem` trait, mount via `winfsp-rs` with `axum+ui` fallback, terminal via ConPTY + busybox shim (Tier 1) and WSL bridge (Tier 2 stretch).

**Tech Stack:** Rust `edition 2021`, `tokio 1`, `axum 0.7`, `windows 0.58`, `winfsp 0.3`, `clap 4`, `egui 0.28` or `xterm.js` + `ui/`, `criterion`, `proptest`, `cargo-fuzz`.

**Spec:** `docs/LINFS_SPEC.md` · **Architecture:** `docs/LINFS_ARCHITECTURE.md` · **Roadmap:** `docs/LINFS_ROADMAP.md`

## Global Constraints

- Rust 95–100% (`cargo run --bin linfs-loc-audit -- --stretch-96` must pass) — no Python/Java product code; `winfsp.sys` is the only allowed C driver.
- `edition 2021`, `resolver = "2"`, `RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-gnu`, MSYS2 bash `C:\msys64\usr\bin\bash.exe -lc`.
- `cargo fmt --all` → `cargo test` → one commit per task/band → `git push` to `https://github.com/platinoff/LinFS` (create `origin` if missing).
- Do not stage `data/*` (except `.gitkeep`), `.env*`, `*.pem`.
- Do not `git add -A`; stage only intended files.
- `--port` default 9998 for preview server (9999 is GSV).

---

### Task 1: Workspace scaffold + xtask + loc-audit

**Files:**
- Create: `Cargo.toml` (workspace), `crates/linfs-core/Cargo.toml`, `crates/linfs-core/src/lib.rs`, `crates/linfs-block/Cargo.toml`, `crates/linfs-fs/Cargo.toml`, `crates/linfs-mount/Cargo.toml`, `crates/linfs-chroot/Cargo.toml`, `crates/linfs-terminal/Cargo.toml`, `crates/linfs-cli/Cargo.toml`, `crates/linfs-gui/Cargo.toml`, `xtask/Cargo.toml`, `xtask/src/main.rs`, `src/bin/linfs_loc_audit.rs`, `.gitignore`, `rust-toolchain.toml`
- Test: `tests/scaffold.rs`

**Interfaces:**
- Consumes: nothing (greenfield)
- Produces: `linfs_core::{Error, Result}` crate used by all later crates; `cargo xtask {products,disk,sync,loc-audit}` commands

- [ ] **Step 1: Write failing scaffold test**

```rust
// tests/scaffold.rs:1
#[test]
fn workspace_members_exist() {
    assert!(std::path::Path::new("crates/linfs-core/src/lib.rs").exists());
    assert!(std::path::Path::new("xtask/src/main.rs").exists());
}
#[test]
fn loc_audit_at_least_96() {
    let out = std::process::Command::new("cargo").args(["run","--bin","linfs-loc-audit","--","--stretch-96"]).output().unwrap();
    assert!(out.status.success(), "loc-audit failed: {}", String::from_utf8_lossy(&out.stderr));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `C:\msys64\usr\bin\bash.exe -lc 'cargo test --test scaffold -v'`
Expected: FAIL — files not found

- [ ] **Step 3: Scaffold workspace**

```toml
# Cargo.toml:1
[workspace]
resolver = "2"
members = ["crates/linfs-core","crates/linfs-block","crates/linfs-fs","crates/linfs-mount","crates/linfs-chroot","crates/linfs-terminal","crates/linfs-cli","crates/linfs-gui","xtask"]
[workspace.package]
edition = "2021"
version = "0.200.0"
license = "MIT"
```

```rust
// crates/linfs-core/src/lib.rs:1
pub mod error;
pub use error::{Error, Result};
```

```rust
// crates/linfs-core/src/error.rs:1
use thiserror::Error;
#[derive(Debug, Error)]
pub enum Error { #[error("io: {0}")] Io(#[from] std::io::Error), #[error("corruption: {0}")] Corruption(String), #[error("unsupported: {0}")] Unsupported(String) }
pub type Result<T> = std::result::Result<T, Error>;
```

```toml
# xtask/Cargo.toml:1
[package] name="xtask" version="0.1.0" edition="2021" publish=false
[dependencies] anyhow="1"
```

```rust
// xtask/src/main.rs:1 — minimal: disk/products/loc-audit subcommands; loc-audit counts .rs lines vs total
```

```toml
# rust-toolchain.toml:1
[toolchain] channel="stable" targets=["x86_64-pc-windows-gnu"]
```

- [ ] **Step 4: Run scaffold test to verify it passes**

Run: `C:\msys64\usr\bin\bash.exe -lc 'cargo test --test scaffold -v'`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
C:\msys64\usr\bin\bash.exe -lc 'git add Cargo.toml rust-toolchain.toml .gitignore xtask/ crates/ src/ tests/scaffold.rs && git commit -m "band 200: scaffold LinFS workspace + xtask + loc-audit"'
```

---

### Task 2: Block layer + partition probe (Band 200 core)

**Files:**
- Create: `crates/linfs-block/src/lib.rs`, `crates/linfs-block/src/block.rs`, `crates/linfs-block/src/win.rs`, `crates/linfs-block/src/image.rs`, `crates/linfs-block/src/partition/mbr.rs`, `crates/linfs-block/src/partition/gpt.rs`, `crates/linfs-block/src/probe.rs`
- Modify: `crates/linfs-cli/src/main.rs:1` — add `linfs list` subcommand
- Test: `crates/linfs-block/tests/partition.rs`, `tests/fixtures/README.md`

**Interfaces:**
- Consumes: `linfs_core::{Error, Result}` from Task 1
- Produces:
  - `linfs_block::Block: ReadAt+WriteAt+Len+SectorSize` (`block.rs:1`)
  - `linfs_block::WinDevice::open(path: &str) -> Result<WinDevice>` (`win.rs:10`)
  - `linfs_block::ImageDevice::open(path) -> Result<ImageDevice>` (`image.rs:10`)
  - `linfs_block::partition::{parse_mbr, parse_gpt} -> Vec<Partition>` (`mbr.rs:1`, `gpt.rs:1`)
  - `linfs_block::probe::probe_fs(block) -> FsType` (`probe.rs:1`) — `enum FsType { Ext4, Xfs, Btrfs, F2fs, Unknown }`

- [ ] **Step 1: Write failing block/partition tests**

```rust
// crates/linfs-block/tests/partition.rs:1
use linfs_block::partition::{parse_mbr, parse_gpt};
#[test]
fn mbr_single_linux_partition() {
    let mut mbr = [0u8; 512];
    mbr[510]=0x55; mbr[511]=0xAA;
    mbr[446+4]=0x83; // Linux
    mbr[446+8]=0x01; mbr[446+9]=0x00; // lba start
    mbr[446+12]=0x20; mbr[446+13]=0x00; // sectors
    let parts = parse_mbr(&mbr).unwrap();
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].ty, 0x83);
}
#[test]
fn gpt_header_crc_rejected() {
    let mut hdr = [0u8; 512];
    hdr[0..8].copy_from_slice(b"EFI PART");
    hdr[510]=0xFF; // corrupt crc
    assert!(parse_gpt(&hdr).is_err());
}
#[test]
fn probe_ext4_magic() {
    use linfs_block::probe::{probe_fs, FsType};
    let mut sb = vec![0u8; 2048];
    sb[1024+56]=0x53; sb[1024+57]=0xEF; // s_magic little-endian 0xEF53 at offset 56
    // wrap in a mock Block
    // assert_eq!(probe_fs(&mock), FsType::Ext4);
}
```

```rust
// crates/linfs-block/tests/block.rs:1
#[test]
fn image_device_read_write() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("raw.img");
    std::fs::write(&p, vec![0u8; 4096]).unwrap();
    let dev = linfs_block::ImageDevice::open(&p).unwrap();
    assert_eq!(dev.len(), 4096);
}
```

- [ ] **Step 2: Run to verify fails**

Run: `C:\msys64\usr\bin\bash.exe -lc 'cargo test -p linfs-block --tests -v'`
Expected: FAIL — modules not defined

- [ ] **Step 3: Implement Block trait + WinDevice + ImageDevice + MBR/GPT + probe**

```rust
// crates/linfs-block/src/block.rs:1
pub trait Block: Send + Sync { fn read_at(&self, off: u64, buf: &mut [u8]) -> std::io::Result<()>; fn write_at(&self, off: u64, buf: &[u8]) -> std::io::Result<()>; fn len(&self) -> u64; fn sector_size(&self) -> u32; }
// SliceBlock for partitions: wraps Arc<dyn Block> + offset+len, delegates with offset
```

```rust
// crates/linfs-block/src/win.rs:1 — CreateFileW + DeviceIoControl via windows crate; fallback to generic error on non-Windows
```

```rust
// crates/linfs-block/src/partition/mbr.rs:1 — validate 0x55AA, 4 entries, CHS ignored
pub struct Partition { pub ty: u8, pub lba_start: u32, pub sectors: u32 }
pub fn parse_mbr(mbr: &[u8;512]) -> linfs_core::Result<Vec<Partition>> { ... }
```

```rust
// crates/linfs-block/src/partition/gpt.rs:1 — "EFI PART", revision, header_size, CRC32 (crc32fast), entries
```

```rust
// crates/linfs-block/src/probe.rs:1
pub enum FsType { Ext4, Xfs, Btrfs, F2fs, Unknown }
pub fn probe_fs(block: &dyn Block) -> FsType {
    let mut buf=[0u8; 4096];
    // ext4 super at offset 1024, magic 0xEF53 at 56
    // xfs super at 0, magic "XFSB"
    // btrfs super at 0x10000, magic "_BHRfS_M" at 64
    // f2fs super at 0, magic 0xF2F52010 LE at 0
}
```

- [ ] **Step 4: Run tests to verify passes**

Run: `C:\msys64\usr\bin\bash.exe -lc 'cargo test -p linfs-block --tests -v'`
Expected: PASS

- [ ] **Step 5: Wire `linfs list` CLI**

```rust
// crates/linfs-cli/src/main.rs:60
#[derive(Parser)] enum Cmd { List, Attach{ path: PathBuf }, Mount{ spec: String, #[arg(long)] drive: Option<String> }, ... }
fn cmd_list() { for dev in enumerate_win_devices() { println!("{}: {} GiB", dev.path, dev.len/1024/1024/1024); for p in dev.partitions { println!("  p{} {:?} {}", p.index, p.fs_type, p.label); } } }
```

Run: `C:\msys64\usr\bin\bash.exe -lc 'cargo run -p linfs-cli -- list -v'`
Expected: prints at least `PhysicalDrive0` (or graceful "no devices" in CI)

- [ ] **Step 6: Commit**

```bash
C:\msys64\usr\bin\bash.exe -lc 'git add crates/linfs-block/ crates/linfs-cli/ && git commit -m "band 200: block layer + MBR/GPT + FS probe + linfs list"'
```

---

### Task 3: LUKS2 + LVM stubs (band 207 slice, ro-first)

**Files:**
- Create: `crates/linfs-block/src/partition/luks.rs`, `crates/linfs-block/src/partition/lvm.rs`
- Test: `crates/linfs-block/tests/luks_lvm.rs`

**Interfaces:**
- Consumes: `Block` from Task 2
- Produces: `LuksBlock::open(block, passphrase) -> Result<LuksBlock>` (`luks.rs:1`), `Lvm::scan(block) -> Vec<LogicalVolume>` (`lvm.rs:1`)

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn luks2_header_parse() {
    let hdr = include_bytes!("../../../tests/fixtures/luks2-header.bin");
    let info = linfs_block::partition::luks::parse_header(hdr).unwrap();
    assert_eq!(info.version, 2);
}
```

- [ ] **Step 2: Run — fail**

Run: `C:\msys64\usr\bin\bash.exe -lc 'cargo test -p linfs-block luks -v'`

- [ ] **Step 3: Implement LUKS2 JSON header parse + argon2id + aes-xts stub (ro decrypt); LVM PV LABELONE scan**

```rust
// luks.rs:1 — parse 4096-byte header, JSON segment, keyslots, verify via argon2 crate; decrypt via aes+xts-mode
```

- [ ] **Step 4: Run — pass**

Run: `C:\msys64\usr\bin\bash.exe -lc 'cargo test -p linfs-block luks_lvm -v'`

- [ ] **Step 5: Commit**

```bash
C:\msys64\usr\bin\bash.exe -lc 'git add crates/linfs-block/src/partition/luks.rs crates/linfs-block/src/partition/lvm.rs && git commit -m "band 207: LUKS2 + LVM scan (ro)"' 
```

> Note: Task 3 may be deferred to band 207; if band 200 time-box hits, mark as `later` and ship band 200 without it.

---

### Task 4: ext4 read-only driver (Band 201)

**Files:**
- Create: `crates/linfs-fs/src/lib.rs`, `crates/linfs-fs/src/vfs.rs`, `crates/linfs-fs/src/ext4/super.rs`, `crates/linfs-fs/src/ext4/group.rs`, `crates/linfs-fs/src/ext4/extent.rs`, `crates/linfs-fs/src/ext4/dir.rs`, `crates/linfs-fs/src/ext4/inode.rs`, `crates/linfs-fs/src/ext4/xattr.rs`
- Test: `crates/linfs-fs/tests/ext4_ro.rs`, `tests/fixtures/*.img` (generated via `scripts/mkfixtures.sh` on Linux)

**Interfaces:**
- Consumes: `Block` from Task 2
- Produces: `linfs_fs::ext4::Fs::open(block) -> Result<Fs>` (`ext4/super.rs:20`), `impl FileSystem for Fs` (`vfs.rs:1`)

- [ ] **Step 1: Write failing ext4 ro test**

```rust
// crates/linfs-fs/tests/ext4_ro.rs:1
#[test]
fn ext4_ls_root() {
    let img = std::path::Path::new("tests/fixtures/ext4-plain.img");
    if !img.exists() { eprintln!("skip: fixture missing"); return; }
    let block = linfs_block::ImageDevice::open(img).unwrap();
    let fs = linfs_fs::ext4::Fs::open(Arc::new(block)).unwrap();
    let entries = fs.readdir(2).unwrap(); // ino 2 = root
    assert!(entries.iter().any(|e| e.name==b"etc"));
    let ino = fs.lookup(2, b"etc").unwrap();
    let attr = fs.getattr(ino).unwrap();
    assert!(attr.is_dir());
}
#[test]
fn ext4_read_etc_hostname() {
    // open, read file, compare bytes
}
```

- [ ] **Step 2: Run — fail**

Run: `C:\msys64\usr\bin\bash.exe -lc 'cargo test -p linfs-fs ext4_ro -v'`

- [ ] **Step 3: Implement super/group/inode/extent/dir/xattr read path**

```rust
// ext4/super.rs:1 — read 1024 B at 1024, validate s_magic 0xEF53, s_inodes_per_group, s_blocks_per_group, feature flags, 64bit high words
// ext4/inode.rs:1 — inode size s_inode_size (128/256), i_mode, i_size, i_blocks, i_block[15] / extent tree
// ext4/extent.rs:1 — walk extent_header (eh_magic 0xF30A), eh_entries, extent leaf (ee_block, ee_len, ee_start_hi/lo)
// ext4/dir.rs:1 — linear dir_entry_2 + htree dx_root/dx_node hash (half_md4) for large dirs
```

- [ ] **Step 4: Run — pass (needs fixtures; generate with `bash scripts/mkfixtures.sh` on Linux or skip if missing)**

Run: `C:\msys64\usr\bin\bash.exe -lc 'cargo test -p linfs-fs ext4_ro -v'`

- [ ] **Step 5: Commit**

```bash
C:\msys64\usr\bin\bash.exe -lc 'git add crates/linfs-fs/ tests/fixtures/ && git commit -m "band 201: ext4 read-only driver"'
```

---

### Task 5: ext4 journal + write (Band 202)

**Files:**
- Create: `crates/linfs-fs/src/ext4/journal.rs`, `crates/linfs-fs/src/ext4/alloc.rs`
- Modify: `crates/linfs-fs/src/ext4/super.rs`, `crates/linfs-fs/src/vfs.rs` — add write methods
- Test: `crates/linfs-fs/tests/ext4_rw.rs`

**Interfaces:**
- Consumes: ext4 ro `Fs` from Task 4
- Produces: `Journal::replay(&mut Fs) -> Result<bool>` (`journal.rs:30`), `Fs::create/write/unlink/rename/chmod` write path via `Journal::transact`

- [ ] **Step 1: Write failing rw test**

```rust
#[test]
fn ext4_create_and_read_back() {
    let dir = tempfile::tempdir().unwrap();
    let img = dir.path().join("rw.img");
    std::fs::copy("tests/fixtures/ext4-plain.img", &img).unwrap();
    let block = Arc::new(linfs_block::ImageDevice::open(&img).unwrap());
    let fs = linfs_fs::ext4::Fs::open(block.clone()).unwrap();
    let ino = fs.create(2, b"hello.txt", 0o644).unwrap();
    fs.write(ino, 0, b"hello linfs").unwrap();
    fs.sync().unwrap();
    drop(fs);
    let fs2 = linfs_fs::ext4::Fs::open(block).unwrap();
    let ino2 = fs2.lookup(2, b"hello.txt").unwrap();
    let mut buf = vec![0u8; 32];
    let n = fs2.read(ino2, 0, &mut buf).unwrap();
    assert_eq!(&buf[..n], b"hello linfs");
}
#[test]
fn ext4_journal_replay_unclean() {
    // fixture with needs_recovery set
    let fs = linfs_fs::ext4::Fs::open(block).unwrap();
    assert!(fs.needs_recovery_replayed());
}
```

- [ ] **Step 2: Run — fail**

Run: `C:\msys64\usr\bin\bash.exe -lc 'cargo test -p linfs-fs ext4_rw -v'`

- [ ] **Step 3: Implement jbd2 replay + Tx**

```rust
// journal.rs:1 — superblock at journal inode (s_journal_inum 8), JBD2_MAGIC 0xC03B3998, descriptor tag, commit block checksum (crc32c)
// alloc.rs:1 — bg_block_bitmap / bg_inode_bitmap alloc/free with buddy
// vfs write dispatch: lookup → alloc inode → alloc blocks (extent) → dir insert → journal Tx { bitmap, dir, inode table, extent } → commit
```

- [ ] **Step 4: Run — pass**

Run: `C:\msys64\usr\bin\bash.exe -lc 'cargo test -p linfs-fs ext4_rw -v'`

- [ ] **Step 5: Commit**

```bash
C:\msys64\usr\bin\bash.exe -lc 'git add crates/linfs-fs/src/ext4/journal.rs crates/linfs-fs/src/ext4/alloc.rs && git commit -m "band 202: ext4 journal replay + write Tx"'
```

---

### Task 6: WinFSP mount + fallback + GUI browser (Band 203)

**Files:**
- Create: `crates/linfs-mount/src/lib.rs`, `crates/linfs-mount/src/winfsp.rs`, `crates/linfs-mount/src/fallback.rs`, `crates/linfs-gui/src/main.rs`, `ui/index.html`, `ui/app.js`, `ui/style.css`
- Modify: `crates/linfs-cli/src/main.rs` — add `mount`/`umount` commands
- Test: `crates/linfs-mount/tests/mount.rs` (requires WinFSP; `#[ignore]` in CI without it)

**Interfaces:**
- Consumes: `FileSystem` from Task 4/5
- Produces: `linfs_mount::Mount::new(fs, drive: &str) -> Result<Mount>` (`winfsp.rs:20`), `linfs_mount::Fallback::serve(fs, addr)` (`fallback.rs:1`)

- [ ] **Step 1: Write failing mount test**

```rust
#[test]
#[ignore] // needs WinFSP
fn winfsp_mount_ls() {
    let fs = open_fixture("ext4-plain.img");
    let m = linfs_mount::Mount::new(fs, "T:").unwrap();
    // std::fs::read_dir("T:\\") should list etc
    let entries = std::fs::read_dir("T:\\").unwrap().count();
    assert!(entries > 0);
    m.unmount().unwrap();
}
```

- [ ] **Step 2: Run — fail**

Run: `C:\msys64\usr\bin\bash.exe -lc 'cargo test -p linfs-mount -- --ignored -v'`

- [ ] **Step 3: Implement winfsp-rs callbacks + axum fallback + GUI**

```rust
// winfsp.rs:1 — impl winfsp::Filesystem, map M:\etc\hostname → "/etc/hostname", translate attrs
// fallback.rs:1 — axum GET /api/fs/* → JSON readdir, GET /api/fs/read?path=... → bytes
// ui/index.html:1 — tree view + Monaco editor (via CDN) + fetch /api/fs
```

- [ ] **Step 4: Run — pass (manual: `cargo run -p linfs-cli -- mount tests/fixtures/ext4-plain.img --drive T` then `dir T:\`)**

Run: `C:\msys64\usr\bin\bash.exe -lc 'cargo test -p linfs-mount -v'`

- [ ] **Step 5: Commit**

```bash
C:\msys64\usr\bin\bash.exe -lc 'git add crates/linfs-mount/ crates/linfs-gui/ ui/ crates/linfs-cli/src/main.rs && git commit -m "band 203: WinFSP mount + fallback browser + linfs mount"'
```

---

### Task 7: chroot + terminal Tier 1 (Band 204 — MVP gate)

**Files:**
- Create: `crates/linfs-chroot/src/lib.rs`, `crates/linfs-chroot/src/root.rs`, `crates/linfs-chroot/src/bind.rs`, `crates/linfs-chroot/src/proc.rs`, `crates/linfs-terminal/src/lib.rs`, `crates/linfs-terminal/src/pty.rs`, `crates/linfs-terminal/src/busybox.rs`
- Modify: `crates/linfs-cli/src/main.rs` — add `chroot`/`exec`/`terminal` commands
- Test: `crates/linfs-chroot/tests/chroot.rs`, `crates/linfs-terminal/tests/pty.rs`

**Interfaces:**
- Consumes: `FileSystem` + `Mount` from Tasks 4–6
- Produces: `linfs_chroot::Root::resolve(cwd, path) -> Resolved` (`root.rs:1`), `linfs_terminal::Pty::spawn(shell, cols, rows)` (`pty.rs:1`), `linfs list` + `linfs chroot` + `linfs exec` CLI

- [ ] **Step 1: Write failing chroot/pty tests**

```rust
// crates/linfs-chroot/tests/chroot.rs:1
#[test]
fn chroot_clamps_dotdot() {
    let root = Root::new(fs_from_fixture("ext4-plain.img"));
    assert_eq!(root.resolve("/", "../../etc").unwrap().as_str(), "/etc");
    assert_eq!(root.resolve("/etc", "../var").unwrap().as_str(), "/var");
}
#[test]
fn chroot_bind_host() {
    let root = Root::new(fs.clone());
    root.bind(Path::new("C:\\tmp"), "/mnt/host").unwrap();
    assert!(root.resolve("/", "/mnt/host/foo").unwrap().is_host());
}
```

```rust
// crates/linfs-terminal/tests/pty.rs:1
#[test]
fn pty_echo() {
    let pty = Pty::spawn("cmd.exe /c echo hi", 80, 24).unwrap();
    let out = pty.read_timeout(Duration::from_secs(2)).unwrap();
    assert!(out.contains("hi"));
}
```

- [ ] **Step 2: Run — fail**

Run: `C:\msys64\usr\bin\bash.exe -lc 'cargo test -p linfs-chroot -p linfs-terminal -v'`

- [ ] **Step 3: Implement Root translator + ConPTY + busybox asset**

```rust
// root.rs:1 — BTreeMap guest→host binds, resolve with clamping, synthetic Proc/Dev inodes
// pty.rs:1 — CreatePseudoConsole + CreateProcessW + pipes (windows crate)
// busybox.rs:1 — include_bytes!("../assets/busybox-x86_64"), shims open/read/write via Root+Fs
```

- [ ] **Step 4: Run — pass**

Run: `C:\msys64\usr\bin\bash.exe -lc 'cargo test -p linfs-chroot -p linfs-terminal -v'`

- [ ] **Step 5: MVP gate script**

Run: `C:\msys64\usr\bin\bash.exe -lc 'cargo run -p linfs-cli -- list && cargo run -p linfs-cli -- chroot tests/fixtures/ext4-plain.img -- cat /etc/hostname'`
Expected: prints fixture hostname

- [ ] **Step 6: Commit**

```bash
C:\msys64\usr\bin\bash.exe -lc 'git add crates/linfs-chroot/ crates/linfs-terminal/ crates/linfs-cli/ && git commit -m "band 204: chroot + ConPTY terminal (MVP gate)"'
```

---

### Future bands (205–212) — outline only

- **Band 205** xfs: add `crates/linfs-fs/src/xfs/*` + `probe Xs->Xfs`, same VFS impl.
- **Band 206** btrfs: `crates/linfs-fs/src/btrfs/*` + chunk/btree/compress.
- **Band 207** f2fs + luks/lvm: already stubbed in Task 3, harden to rw.
- **Band 208** WSL bridge: `crates/linfs-terminal/src/wsl.rs` + autodetect.
- **Band 209** bulk ops/undo: `linfs-fs/src/journal` undo log + GUI drag-drop.
- **Band 210** fsck/perf: `linfs-cli fsck`, `benches/ls_r.rs` criterion.
- **Band 211** qcow2/vhd: `crates/linfs-block/src/image/qcow2.rs`.
- **Band 212** release: `build.rs` installer, `README` gif, `cargo xtask live`.

---

## Verification Checklist (run before each band commit)

- `C:\msys64\usr\bin\bash.exe -lc 'cargo fmt --all'`
- `C:\msys64\usr\bin\bash.exe -lc 'cargo test -v'`
- `C:\msys64\usr\bin\bash.exe -lc 'cargo clippy -- -D warnings'`
- `C:\msys64\usr\bin\bash.exe -lc 'cargo run --bin linfs-loc-audit -- --stretch-96'`
- `C:\msys64\usr\bin\bash.exe -lc 'cargo run -p linfs-cli -- list'`

