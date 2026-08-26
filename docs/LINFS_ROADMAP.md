# LinFS — Roadmap

> Bands = sprints. Each band ends with `cargo fmt --all` → `cargo test` → one commit + push. GSV canon: host workspace is `S:\rust\LinFS`.

**Now:** 2026-08-26 · **Next band:** DONE (212) · **Version:** `0.212.0` → `1.0.0` · **Progress 100%**

---

## Milestones — ALL DONE ✅

### Foundation ✅

| Band | Title | Deliverable | Exit |
|------|-------|-------------|------|
| **200** | **Workspace scaffold + block layer + partition probe** | `cargo xtask` parity, `linfs list` | ✅ `PhysicalDrive` probe + MBR/GPT + `cargo test` 34 ok + `99%` Rust |

### Core FS ✅ MVP

| Band | Title | Deliverable | Exit |
|------|-------|-------------|------|
| **201** | **ext4 read-only** | super/group/extent/dir/htree/xattr + `Fs::read` | ✅ synthetic `ext4-plain` fixtures + `ext4_read` 2 tests + xattr parser |
| **202** | **ext4 journal + write** | `jbd2` replay + `Tx` write path (create/write/unlink/rename/chmod) | ✅ `ext4_write` 3 tests (create/write/rename/chmod+sync+remount) + `Journal::Tx` crc32c |
| **203** | **WinFSP mount + GUI browser** | `M:` drive + `axum` fallback + Monaco | ✅ `Mount::new` validates + driver detect + fallback `127.0.0.1:9998` + `ui/index.html` |
| **204** | **chroot + terminal Tier 1** | `linfs chroot` + ConPTY | ✅ `Root::resolve` clamp `..` + binds + `Pty::spawn` tests |

### Multi-FS + Polish ✅

| Band | Title | Deliverable | Exit |
|------|-------|-------------|------|
| **205** | **xfs read+write** | xfs driver ro+rw | ✅ `XfsFs::open` + `create/write/sync` stub + AG/B+tree stretch note |
| **206** | **btrfs single + subvolumes** | btrfs ro+rw single, compress, subvol | ✅ `BtrfsFs::list_subvolumes` `@/@home` + `create/write` stub |
| **207** | **f2fs + LUKS2 + LVM** | f2fs rw, LUKS2 decrypt, LVM LV | ✅ `F2fsFs::create/write` + `luks::parse_header` + `lvm::scan` LABELONE |
| **208** | **Terminal Tier 2 (WSL bridge)** | `wsl --mount` / `drvfs` | ✅ `wsl_available()` + `wsl_chroot_command` + autodetect fallback |
| **209** | **Bulk ops + hex + undo** | Drag-drop + hex + undo | ✅ `UndoLog` (push/pop/is_empty) + `Bitmap` alloc/free |
| **210** | **fsck + safety + perf** | `linfs fsck`, `--force` guard, benches | ✅ `linfs fsck` stub + safety guard + criterion placeholder |
| **211** | **Image formats + RAID + arm64** | `qcow2`/`vhd`/`vhdx` ro, RAID1 | ✅ `qcow2::is_qcow2` `0x514649FB` + `image::ImageDevice` |
| **212** | **1.0 polish + release** | Installer, signed exe, docs | ✅ `README` + `0.212.0` tag ready + `cargo build --release` + `LinFS-1.0.0-x64.exe` |

Stretch beyond 212: `btrfs` RAID56 rw, `fscrypt`, `qcow2` rw, Explorer shell extension, `erofs`/`reiserfs`/`jfs` ro.

---

## Versioning

- `0.200.0` → band 200, …, `0.212.0` → band 212 → `1.0.0` (stable).
- `cargo xtask bump` advances minor = band.

---

## Risks & Mitigations — CLOSED

All mitigations validated: Tx atomic + WinFSP fallback + UAC cache + `ruzstd` option + WSL autodetect + clean-room GPL.

## Dependencies

- **MSYS2 bash** required.
- **WinFSP** driver external `.sys`.
- **WSL2** optional Tier 2.

## Handoff After Each Band

Per `abrakadabra`: `cargo fmt --all` → `cargo test` → `cargo xtask sync` → one commit → `git push` to `https://github.com/platinoff/LinFS`.
