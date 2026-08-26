# Промпт наступної сесії (LinFS)

**Оновлено:** 2026-08-26 (band **201** · next **202**)

**Workspace:** відкрити Cursor на **`S:\rust\GSV`** або `gsv.code-workspace` → `абракадабра` / `abrakadabra` → вибрати **`linfs`** (registered `rust`).

```
абракадабра
```

**Порядок:** `cargo xtask products` → обрати `linfs` → S0 диск (`cargo xtask disk`) → project scan (warnings first) → drain band **202**.

## Band стан

- **band 200 ✅** — workspace scaffold + xtask + `linfs-block` MBR/GPT/probe/ImageDevice/WinDevice stub + `linfs-cli list` + `linfs loc-audit` + git push `platinoff/LinFS:main` (`128df6c`). Spec/Architecture/Roadmap/Plan docs present.
- **band 201 ✅** — ext4 read-only driver: `linfs-fs/src/ext4/{superblock,group,extent,dir,inode,xattr}` + `Fs::read` (extent logical→phys + unwritten/hole zero-fill) + `xattr` parser + clippy clean + `99%` Rust + `ext4_read` synthetic tests.
- **band 202 NEXT** — ext4 journal (jbd2 replay) + write Tx (create/write/unlink/rename/chmod).
- **band 202** — ext4 journal (jbd2 replay) + write Tx (create/write/unlink/rename/chmod).
- **band 203** — WinFSP mount (`M:`) + `axum` fallback + GUI browser (`ui/` + Monaco + `crates/linfs-gui`).
- **band 204** — chroot (`Root::resolve` clamp `..`) + ConPTY terminal Tier 1 (busybox) → **MVP gate** (spec §5).
- **bands 205–212** — xfs → btrfs → f2fs/LUKS/LVM → WSL bridge → bulk/undo → fsck/perf → qcow2/RAID → 1.0 release. See `docs/LINFS_ROADMAP.md`.

**Канон:** Rust **95–100%** (`--stretch-96`), без Python/Java, MSYS2 bash, `cargo fmt` → `cargo test` → один commit → `git push`.

Close: `cargo fmt --all` → `cargo test` → `git commit` → `git push` origin main.

Spec: `docs/LINFS_SPEC.md` · Arch: `docs/LINFS_ARCHITECTURE.md` · Kit: `S:/rust/GSV/.agents/skills/abracadabra/SKILL.md`
