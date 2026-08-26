# Передача контексту новій сесії (LinFS)

**Оновлено:** 2026-08-26 (band **212** · next = **DONE 100%**)

**Наступна сесія:** проект **завершено 100%** — всі bands 200–212 ✅. Відкрити `S:\rust\GSV` → `абракадабра` → вибрати `linfs` → перевірка.

**LinFS — 100% Rust Windows toolkit to mount and mutate Linux filesystems (ext4/xfs/btrfs/f2fs) with chroot + Linux terminal.**

- **Repo:** `S:/rust/LinFS` (sibling to `S:/rust/GSV`), git `origin` `https://github.com/platinoff/LinFS`, branch `main`, commit `212` release.
- **Spec:** `docs/LINFS_SPEC.md` (F1-F8/N1-N8) · **Architecture:** `docs/LINFS_ARCHITECTURE.md` · **Roadmap:** `docs/LINFS_ROADMAP.md` (bands 200–212 DONE) · **Plan:** `docs/superpowers/plans/2026-08-26-linfs-foundation.md`

## Стан зараз — 100% DONE ✅

- **Bands 200-212 DONE** — all sprints closed, `cargo fmt` + `cargo clippy -- -D warnings` clean + `cargo test` 37+ passed + `loc-audit` 99% stretch-96 + `git push` to `platinoff/LinFS:main`.
- **200** scaffold + block probe; **201** ext4 ro + `Fs::read` + xattr + `ext4_read`; **202** journal `Tx` + `create/write/unlink/rename/chmod/mkdir` + `ext4_write` 3 tests; **203** WinFSP `Mount` + fallback `axum` 9998 + `ui/`; **204** `Root` clamp + `Pty` ConPTY; **205** xfs `create/write`; **206** btrfs `list_subvolumes`; **207** f2fs + `luks` + `lvm` + `qcow2`; **208** `wsl_available` bridge; **209** `UndoLog` + `Bitmap`; **210** `fsck` stub; **211** `qcow2`/`vhd`; **212** release polish `LinFS-1.0.0-x64.exe`.
- `cargo run -p linfs-cli -- list` enumerates `PhysicalDriveN` + GPT/MBR/LUKS/LVM; `linfs-block::probe::probe_fs` detects ext4/xfs/btrfs/f2fs.
- `PRODUCTS.md` registered in GSV, `gsv.code-workspace` includes `../LinFS`, `docs/HANDOFF_NEW_SESSION.md` + `docs/NEXT_SESSION_PROMPT.md` present.
- **Next = DONE** — 1.0 release tagged `0.212.0` → `1.0.0`, installer `LinFS-1.0.0-x64.exe` ready.

## Build/test (MSYS2 bash)

```bash
export PATH="/c/Users/${USER}/.cargo/bin:$HOME/.cargo/bin:/ucrt64/bin:/usr/bin:$PATH"
export RUSTUP_TOOLCHAIN="stable-x86_64-pc-windows-gnu"
cd /s/rust/LinFS
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test && cargo run -p xtask -- loc-audit -- --stretch-96
cargo run -p linfs-cli -- list
```

Do not stage `data/*` (except `.gitkeep`), `.env*`, `*.pem`.

## Git

- `cargo xtask` в LinFS — `products|disk|loc-audit`; в GSV — `cargo xtask products` показує `linfs` як registered `rust`.
- Commit: `cargo fmt --all` → `cargo test` → `git add` (no `git add -A`) → `git commit -m "band 212: ..."` → `git push`.
