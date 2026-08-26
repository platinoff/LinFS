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

## Safety

- `needs_recovery` journal gate — refuse rw if replay failed
- No write to Disk 0 without `--force`
- All superblock/B+tree parsers fuzzed; `cargo audit`/`deny` in CI
