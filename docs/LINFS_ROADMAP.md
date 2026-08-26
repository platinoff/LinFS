# LinFS — Roadmap

> Bands = sprints. Each band ends with `cargo fmt --all` → `cargo test` → one commit + push. GSV canon: host workspace is `S:\rust\LinFS`.

**Now:** 2026-08-26 · **Next band:** 200 · **Version:** `0.200.0` (band == minor, per GSV `semver minor = band`).

---

## Milestones (Now / Next / Later)

### Now — Band 200: Foundation (2 weeks)

| Band | Title | Deliverable | Exit Criteria |
|------|-------|-------------|---------------|
| **200** | **Workspace scaffold + block layer + partition probe** | `cargo xtask` parity with GSV, `linfs list` enumerates disks/partitions | `linfs list` on a real Windows box prints `PhysicalDrive0/1/2` + GPT/MBR partitions + FS type probe (`ext4`/`xfs`/`btrfs`/`f2fs`/`unknown`). `cargo test` passes partition parser unit tests + `cargo loc-audit` ≥96% Rust. |

Scope pushed to plan `docs/superpowers/plans/2026-08-26-linfs-foundation.md` Task 1–3.

---

### Next — Bands 201–204: Core FS (6 weeks) — **MVP**

| Band | Title | Deliverable | Exit Criteria |
|------|-------|-------------|---------------|
| **201** | **ext4 read-only** | `linfs-fs` ext4 driver ro: super/group/extent/dir/htree/xattr | Mount 5 fixtures (`ext4-plain`, `ext4-64bit`, `ext4-extent`, `ext4-htree-100k`, `ext4-inline`) → `linfs ls` matches `debugfs ls` output byte-for-byte. `cargo fuzz` 10 min no crash. |
| **202** | **ext4 journal + write** | `jbd2` replay + `Tx` write path (create/write/unlink/rename/chmod) | `linfs cat` / `linfs edit` / `linfs mkdir` on rw mount survives `sync` + remount; `needs_recovery` fixture replays; `e2fsck -n` reports clean. |
| **203** | **WinFSP mount + GUI browser** | `M:` drive + internal `axum` browser + Monaco editor | `notepad M:\etc\hostname` edits round-trip; GUI lists permissions/owner/symlink; copy Windows→LinFS preserves mode. No WinFSP → fallback browser still works. |
| **204** | **chroot + terminal Tier 1** | `linfs chroot M: -- sh` + `linfs exec` + ConPTY widget | `chroot M: /bin/sh -c 'cat /etc/hostname'` returns edited value; `..` at `/` clamped; `/proc/mounts` + `/dev/null` synthetic. **MVP gate** (spec §5) passes for `ext4`. |

MVP demo script (band 204 exit):
```bat
linfs list
linfs attach E:\rpi.img          :: or \\.\PhysicalDrive2
linfs mount 2:2 --rw --drive M
linfs ls M:/etc
linfs chroot M: -- cat /etc/hostname
linfs terminal --root M:         :: opens ConPTY busybox sh, `echo hi > /tmp/x && cat /tmp/x`
notepad M:\etc\hostname           :: edit, save
linfs umount M: && linfs detach 2
:: re-attach on Linux, cat confirms edit
```

---

### Later — Bands 205–212: Multi-FS + Polish (8 weeks)

| Band | Title | Deliverable | Exit Criteria |
|------|-------|-------------|---------------|
| **205** | **xfs read+write** | xfs driver ro+rw (AG, B+tree, log replay) | Same MVP gate but `xfs` fixture: edit + chroot cat. |
| **206** | **btrfs single + subvolumes** | btrfs ro+rw single, compress read, subvol list | Mount `btrfs-single.img` with `compress=zstd`, browse subvol `@` + `@home`, edit survives `btrfs check`. |
| **207** | **f2fs + LUKS2 + LVM** | f2fs rw, LUKS2 decrypt, LVM LV mount | Attach LUKS2 `ext4` → passphrase prompt → rw mount; LVM `vg0/lv_root` appears as mountable partition. |
| **208** | **Terminal Tier 2 (WSL bridge)** | `wsl --mount` / `drvfs` interop, autodetect | If WSL present, `linfs chroot M: -- /bin/bash` runs guest's real bash (test `apt list`); else busybox fallback. `Auto` mode documented. |
| **209** | **Bulk ops + hex + undo** | Drag-drop Windows↔LinFS, hex editor, per-mount undo | Drag 1 GiB folder Windows→M: preserves modes; hex edit binary; `Undo` rolls back last `Tx`. |
| **210** | **fsck + safety + perf** | `linfs fsck`, `--force` guard for Disk 0, `criterion` benches | `linfs fsck 2:2` matches `e2fsck -n`; `ls -R 100k` <2 s; `cargo fuzz` 1 h clean; `cargo audit`/`deny` pass. |
| **211** | **Image formats + RAID + arm64 stretch** | `qcow2`/`vhd`/`vhdx` ro, mdadm RAID1 read, `aarch64` qemu-user stub | Mount `rpi.qcow2` ro; RAID1 degraded still reads. |
| **212** | **1.0 polish + release** | Installer (`winfsp.msi` bundled), signed `.exe`, docs, `cargo xtask live` | `cargo build --release` → `LinFS-1.0.0-x64.exe` installer; `README` with MVP demo gif; `0.212.0` tag + GitHub release. |

Stretch beyond 212: `btrfs` RAID56 rw, `fscrypt`, `qcow2` rw, Explorer shell extension, `erofs`/`reiserfs`/`jfs` ro, `netfs`.

---

## Versioning

- `0.200.0` → band 200, `0.201.0` → band 201, …, `0.212.0` → band 212 → `1.0.0` (stable).
- `cargo xtask bump` advances minor = band, same as GSV/poolAI canon.

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| ext4 journal corruption on power loss mid-Tx | Journal `Tx` is atomic: commit block last with checksum; checkpoint only after commit. Replay is idempotent. |
| WinFSP not installed on user machine | Fallback internal browser (band 203) + bundle `winfsp.msi` silent install on first `mount`. |
| UAC fatigue from raw disk open | Cache elevation token per session; image files never elevate; offer ro without elevation. |
| `btrfs` zstd needs C `zstd-sys` (breaks pure-Rust ratio) | Use `zstd` crate (wraps C but tiny) — ratio still ≥96%; gate via `loc-audit --allow-native zstd-sys`. Alternative `ruzstd` pure Rust stretch. |
| WSL not present (Tier 2 unavailable) | Tier 1 busybox always ships; Tier 2 is optional autodetect. |
| GPL contamination from kernel code | Clean-room from specs + MIT crates (`ext4-view`, `btrfs` crates) — never copy `fs/ext4/*.c`. |

---

## Dependencies on Environment

- **MSYS2 bash** required (same as GSV) — all `cargo`/`git` via `C:\msys64\usr\bin\bash.exe -lc`.
- **WinFSP** driver — external `.sys`, not counted in Rust ratio.
- **WSL2** — optional, only for Tier 2 terminal.

---

## Handoff After Each Band

Per `abrakadabra` skill: `cargo fmt --all` → `cargo test` (here; PoolAI uses `cargo test-ci`) → `cargo xtask record-speed`/`record-rust` → `cargo xtask sync` → one commit in `S:\rust\LinFS` → `git push` to `origin` (`https://github.com/platinoff/LinFS` — create if missing).

