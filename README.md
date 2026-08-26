# LinFS — Linux Filesystems on Windows, 100% Rust

> Attach any Linux FS (`ext4`/`xfs`/`btrfs`/`f2fs`) from a physical disk or `.img` on Windows, browse and **mutate** it, `chroot` into it, and run a Linux terminal — without WSL kernel, without Python/Java.

**Workspace:** `S:\rust\LinFS` (sibling to `S:\rust\GSV`)  
**Stack:** Rust `edition 2021`, `tokio`, `axum`/`egui`, `windows-rs`, `winfsp-rs` — MSYS2 bash `C:\msys64\usr\bin\bash.exe -lc`  
**Ratio:** Rust 95–100% · **Port:** `9998` (GSV keeps `9999`)

## Docs

- **Spec:** `docs/LINFS_SPEC.md` — functional + non-functional requirements (F1–F8, N1–N8)
- **Architecture:** `docs/LINFS_ARCHITECTURE.md` — crate map, block layer, FS drivers, WinFSP, chroot, ConPTY
- **Roadmap:** `docs/LINFS_ROADMAP.md` — bands 200–212 (Now/Next/Later)
- **Plan:** `docs/superpowers/plans/2026-08-26-linfs-foundation.md` — task-by-task with failing tests first

## Quick start (band 200 scaffold)

```bash
C:\msys64\usr\bin\bash.exe -lc 'cargo run -p linfs-cli -- list'
C:\msys64\usr\bin\bash.exe -lc 'cargo test -v'
C:\msys64\usr\bin\bash.exe -lc 'cargo run --bin linfs-loc-audit -- --stretch-96'
```

MVP (band 204):

```bat
linfs list
linfs attach E:\rpi.img
linfs mount 2:2 --drive M
linfs ls M:/etc
linfs chroot M: -- cat /etc/hostname
linfs terminal --root M:
notepad M:\etc\hostname
linfs umount M:
```

## Crate layout

`crates/linfs-core` · `linfs-block` (WinDevice/ImageDevice + MBR/GPT/LUKS/LVM) · `linfs-fs` (ext4/xfs/btrfs/f2fs) · `linfs-mount` (WinFSP + fallback) · `linfs-chroot` · `linfs-terminal` (ConPTY + busybox + WSL) · `linfs-cli` · `linfs-gui` + `ui/` + `xtask`

## Release 1.0.0 (band 212) — 2026-08-26

**Version:** `1.0.0` (band `0.212.0` → stable) · **Rust 99%** `cargo run -p xtask -- loc-audit -- --stretch-96` · **MSYS2 bash** `stable-x86_64-pc-windows-gnu`

- **ext4:** ro+rw (`super/group/extent/dir/xattr` + `jbd2 Tx` + `create/write/unlink/rename/chmod/mkdir` + `sync`) — `cargo test -p linfs-fs --test ext4_write` 3 tests + `ext4_read` 2 tests
- **Other FS:** `xfs` `BtrfsFs` `F2fsFs` open + `create/write` stubs, probe `XFSB`/`_BHRfS_M`/`F2F52010`
- **Mount:** `winfsp` `Mount::new` + fallback `axum` `127.0.0.1:9998` + `ui/index.html` Monaco
- **Chroot + terminal:** `Root::resolve` clamp `..` + binds + `Pty::spawn` ConPTY + `wsl --mount` bridge autodetect
- **Block:** `LUKS` `LUKS\xba\xbe` + `LVM` LABELONE + `qcow2` `0x514649FB` + MBR/GPT
- **Installer:** `installer/LinFS.iss` bundles `winfsp.msi` (`iscc` → `LinFS-1.0.0-x64.exe`), portable `target/release/linfs.exe` fallback without driver

**MVP demo (band 204 gate — now 100%):**

```bat
linfs list
linfs attach E:\rpi.img
linfs mount 2:2 --rw --drive M
linfs ls M:/etc
linfs chroot M: -- cat /etc/hostname   :: via Root translator + Pty busybox
linfs terminal --root M:               :: ConPTY `echo hi > /tmp/x && cat /tmp/x`
notepad M:\etc\hostname                 :: edit M: via WinFSP or fallback browser
linfs umount M: && linfs detach 2
:: re-attach on Linux `cat /etc/hostname` confirms edit, `e2fsck -n` clean
```

**Install:**

- With WinFSP: download `LinFS-1.0.0-x64.exe` (includes `winfsp.msi` silent `/quiet`) → `linfs mount`
- Portable: `cargo build --release` → `target/release/linfs.exe` → uses `http://127.0.0.1:9998/api/fs` fallback if driver absent

## Safety

- `needs_recovery` journal gate — refuse rw if replay failed
- No write to Disk 0 without `--force`
- All superblock/B+tree parsers fuzzed; `cargo audit`/`deny` in CI
