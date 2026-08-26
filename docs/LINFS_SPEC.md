# LinFS — Spec

> **Conception:** 100% Rust Windows-native app that attaches Linux filesystems from any connected block device or image, provides read/write file manipulation, `chroot` semantics, and an integrated Linux terminal — without requiring WSL kernel or Python/Java.

**Product id:** `linfs` (sibling to `gsv`, `poolai`)  
**Platform:** Windows 10 22H2+ / Windows 11, x64 (arm64 stretch). **MSYS2 bash + `stable-x86_64-pc-windows-gnu`.**  
**Ratio canon:** Rust 95–100% / 0–5% generated glue — `cargo run --bin gsv-loc-audit` equivalent `linfs-loc-audit`.  
**Server port (if GUI):** 8765 is reserved — use 9998 for LinFS preview server if needed (GSV keeps 9999).

---

## 1. Vision

Windows has no native `ext4/xfs/btrfs/f2fs` write support. Existing tools (ext2fsd, Paragon, WSL `--mount`) are kernel drivers, incomplete write paths, or WSL-bound. **LinFS is a user-space, pure-Rust filesystem toolkit** that:

1. Enumerates every attachable block source on the machine.
2. Opens it **without reformatting**, parses partition + FS, and mounts it in user-space.
3. Lets the user browse **and mutate** the tree (create/edit/delete, permissions, symlinks, xattrs).
4. Enters a **chroot** into that FS and drops into a **Linux shell** (busybox + optionally the guest's own `/bin/bash`) where every syscall operates on the attached tree.
5. Unmounts cleanly, flushes journal, and detaches.

Destination: *modificate files inside Linux filesystems* — e.g. fix a broken Raspberry Pi SD card, edit `/etc/fstab` on a dead Ubuntu SSD, recover Docker layers from `ext4` on Windows.

---

## 2. Functional Requirements

### F1 — Device Attach
- `F1.1` Enumerate `\\.\PhysicalDriveN`, `\\.\Volume{GUID}`, USB mass storage arrivals (WM_DEVICECHANGE), SD readers, NVMe, and `*.img/*.raw/*.qcow2/*.vhd/*.vhdx` files via file picker.
- `F1.2` Open handles with `CreateFileW` (`GENERIC_READ|WRITE`, `FILE_SHARE_READ|WRITE`, `OPEN_EXISTING`), `IOCTL_DISK_GET_DRIVE_GEOMETRY_EX`, `IOCTL_DISK_GET_LENGTH_INFO`, `FSCTL_ALLOW_EXTENDED_DASD_IO`. No admin bypass — request elevation once, cache token.
- `F1.3` Snapshot safety: if device is online/mounted by Windows, offer **offline** (`IOCTL_VOLUME_OFFLINE`) or **read-only** fallback, never corrupt.
- `F1.4` Attach by: raw disk N, partition slice (`Disk 2 / Partition 3`), or image file (with optional offset). Remember recent attaches.

### F2 — Partition & Volume Discovery
- `F2.1` Parse MBR + GPT (protective MBR, `0xEE`), including hybrid; list partitions with type GUID, label, offset, length, flags (boot/esp/lvm).
- `F2.2` Detect LVM2 PV → VG → LV, LUKS1/LUKS2 (prompt passphrase, `argon2`), mdadm RAID 0/1/5 (read-only if degraded), and `dm-crypt` plain.
- `F2.3` Detect FS superblock per partition/LV: `ext2/3/4`, `xfs`, `btrfs`, `f2fs`, `erofs`, `reiserfs` (ro), `jfs` (ro), `bfs` stretch. Unknown → hex viewer.
- `F2.4` Show tree before mount: `Disk 2 (Samsung 870) → p2 ext4 "rootfs" 112 GiB → Mount`.

### F3 — Linux Filesystem Read/Write (core)
- `F3.1` `ext4` — **full RW is P0** (journal `jbd2` replay, extents, `flex_bg`, `meta_bg`, `64bit`, `dir_index` htree, `extent` tree, `xattr`, `acl`, `symlink` inline vs block). Handle `needs_recovery` flag: replay before write; mark clean on unmount.
- `F3.2` `xfs` — RW (AGs, B+trees, `rmapbt`, `reflink` read, `log` replay). `btrfs` — RW single-profile; RAID1/DUP read; compress `zlib/lzo/zstd` read+write; subvolumes+snapshots.
- `F3.3` `f2fs` — RW (NAT/SIT, checkpoint replay). Others ro initially, rw stretch.
- `F3.4` Inode ops: `lookup`, `create`, `mkdir`, `unlink`, `rmdir`, `rename` (cross-dir), `symlink`, `readlink`, `link` (hard), `chmod`, `chown`, `utimes`, `truncate`, `fallocate` (punch hole for ext4/xfs).
- `F3.5` Extended attributes: `user.*`, `security.*`, `trusted.*`, `system.posix_acl_*` preserved. SELinux `security.selinux` round-trips.
- `F3.6` Case sensitivity: preserve Linux semantics (case-sensitive) even though host is case-insensitive — no folding.
- `F3.7` Large files: `>4 GiB`, sparse files, `>255` char names (ext4 `long name`). No silent truncation.

### F4 — Mount Exposure
- `F4.1` Primary: **WinFSP** virtual drive (`M:` or mountpoint `C:\mnt\linfs\<label>\`). Implemented via `winfsp-rs` bindings — the only non-Rust component is the signed `winfsp-x64.sys` already on the machine; Rust drives it.
- `F4.2` Fallback: internal virtual FS (no drive letter) browsed in the LinFS GUI (tree + hex) when WinFSP absent.
- `F4.3` Options per mount: `ro/rw`, `noexec` (for host), `noatime`, `commit=60`. Guard: refuse rw mount if `needs_recovery` replay failed.

### F5 — File Mutation UI (destination)
- `F5.1` Explorer-like browser: tree, permissions `rwxrwxrwx`, owner `uid:gid` + name map from guest `/etc/passwd`, symlink targets, size, mtime.
- `F5.2` Edit file: open in embedded Monaco editor (read bytes → UTF-8/hex), save → write back via FS driver (journal tx). Binary files: hex editor.
- `F5.3` Bulk ops: drag-drop from Windows ↔ LinFS (copies via userspace, preserves mode), delete, rename, `chmod -R`, `chown -R`.
- `F5.4` Undo: per-mount journal of mutations (until unmount/commit). `Discard` rolls back last tx.

### F6 — chroot
- `F6.1` `linfs chroot <mount>` — re-root `"/"` to the mounted tree for subsequent ops. Conceptually `chroot(2)` but implemented as **path translation layer**: every absolute path resolves under `<mount>`; `..` at `/` stays at `/`. Two callers:
  - (a) Internal file ops (already chrooted).
  - (b) The Linux terminal (F7) — its `cwd` and `exec` see the guest root.
- `F6.2` Bind mounts: `--bind /windows/host/path /mnt/host` (expose Windows folder inside chroot) and `--bind /proc` / `/sys` / `/dev` synthetic (minimal stub: `/proc/mounts`, `/dev/null/zero/random`).
- `F6.3` Isolation: Windows host FS is **not** writable from inside chroot unless explicitly bound. Escape via `../` blocked.
- `F6.4` Multiple chroots: each mount has independent root; switching is instant (just re-point translator).

### F7 — Linux Terminal
- `F7.1` Integrated terminal (ConPTY on Windows) that speaks Linux. Tiered implementation:
  - **Tier 1 (MVP):** `busybox` (static) + `musl` shipped as Rust `include_bytes!`, executed via **ELF loader + syscall shim** in userspace (`linfs-exec` crate) — enough for `sh`, `ls -l`, `chmod`, `vi`, `passwd` edits. No WSL needed.
  - **Tier 2:** If WSL2 present, option to `wsl --mount --bare` bridge + `wsl chroot /mnt/linfs/rootfs /bin/bash` — real guest binaries run under WSL kernel, files backed by LinFS's WinFSP mount (or direct block via `\\.\PhysicalDrive` passthrough).
  - **Tier 3 (stretch):** `qemu-user` style emulation for foreign arch (`aarch64` rootfs on x64 Windows).
- `F7.2` Terminal must: support `TERM=xterm-256color`, Unicode, PTY resize, copy/paste, scrollback 100k lines, and environment `PATH=/bin:/usr/bin TERM=... HOME=/root`.
- `F7.3` `chroot` is implicit: launching terminal with a mounted root auto-`chroot`s; `exit` returns to Windows shell but stays in app.
- `F7.4` Persistence: terminal history per chroot saved under `data/linfs/history/<id>`.

### F8 — Detach & Safety
- `F8.1` On detach: `sync`, checkpoint, clear `needs_recovery` only after successful commit, flush WinFSP, close handle, `IOCTL_VOLUME_ONLINE` if we offlined.
- `F8.2` Crash safety: if app crashes mid-tx, next mount replays journal — no FS corruption beyond what the Linux kernel would allow.
- `F8.3` `fsck` integration: bundle `e2fsck` (Rust port) / call external `fsck.ext4` if present; offer "Check before mount" toggle.

---

## 3. Non-Functional Requirements

| ID | Requirement | Target |
|----|-------------|--------|
| N1 | **100% Rust** | `cargo loc-audit` ≥95% Rust, 0 Python/Java. Only Rust crates + `winfsp` driver (C sys) + `windows-rs` bindings. Bash scripts only for `cargo xtask`. |
| N2 | **No admin except when needed** | Raw disk open degrades to ro without elevation; image files need no admin. UAC prompt once per attach. |
| N3 | **Perf** | `ls -R` 100k files < 2 s on SSD; `dd if=/dev/zero of=big bs=1M count=1024` ≥ 80% native speed (WinFSP overhead). |
| N4 | **Safety** | Never write to a partition whose type != Linux FS unless user confirms. Fuzz all parsers (`cargo fuzz`). |
| N5 | **Offline** | Works with no network; no telemetry. |
| N6 | **Windowed + CLI** | GUI (`egui`/`tauri` with Rust backend OR `axum` local server + `ui/` thin JS — reuse GSV pattern) + `linfs.exe` CLI for scripting. |
| N7 | **Test** | `cargo test` + `cargo test --features integration` with loopback images; property tests for rename/journal idempotence. |
| N8 | **License** | MIT; FS code derived from clean-room or permissive crates, not GPL-v2-only kernel copy-paste. |

---

## 4. Out of Scope (v1)
- Network filesystems (NFS/CIFS) — local block only.
- Encrypted FS write for `fscrypt` (ro only).
- Boot loader install (`grub-install`) — file edits only, not MBR write.
- Windows Shell namespace extension (Explorer right-click) — stretch.

---

## 5. Acceptance Criteria (MVP gate)
- Attach a USB stick with `ext4` (created via `mkfs.ext4` on Linux) on Windows, browse `/etc`, edit `/etc/hostname` in embedded editor, `chmod`, `chroot` + `cat /etc/hostname` in LinFS terminal shows the edit, detach, re-attach on Linux and `cat` confirms.
- Same flow for `btrfs` single and `xfs`.
- `ext4` with `needs_recovery` (unclean) is detected and replayed.
- LUKS2 `ext4` prompts passphrase and mounts rw.
- WinFSP drive `M:` shows the same tree to `notepad.exe`.

---

## 6. Open Questions
- WinFSP redistribution: bundle installer or require user pre-install? → Bundle `winfsp.msi` silent install on first mount, else fallback to internal browser.
- `btrfs` zstd write: include `zstd-rs` or shell to `zstd-sys` (C)? Prefer `zstd-rs` pure Rust (`zstd` crate wraps C still — gate on audit).
- Terminal Tier 1 vs Tier 2 default: autodetect WSL — if present default to Tier 2 (real kernel), else Tier 1.
