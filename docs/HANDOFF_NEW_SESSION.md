# Передача контексту новій сесії (LinFS)

**Оновлено:** 2026-08-26 (band **200** · next = **201**)

**Наступна сесія:** відкрити Cursor на **`S:\rust\GSV`** (або `gsv.code-workspace`) →
`абракадабра` / `abrakadabra` → `cargo xtask products` → вибрати **`linfs`** → S0 диск/git → project scan.

**LinFS — 100% Rust Windows toolkit to mount and mutate Linux filesystems (ext4/xfs/btrfs/f2fs) with chroot + Linux terminal.**

- **Repo:** `S:/rust/LinFS` (sibling to `S:/rust/GSV`), git `origin` `https://github.com/platinoff/LinFS`, branch `main`, commit `128df6c` band 200 scaffold.
- **Spec:** `docs/LINFS_SPEC.md` (F1-F8/N1-N8) · **Architecture:** `docs/LINFS_ARCHITECTURE.md` · **Roadmap:** `docs/LINFS_ROADMAP.md` (bands 200–212) · **Plan:** `docs/superpowers/plans/2026-08-26-linfs-foundation.md`
- **Workspace:** `resolver 2`, crates `linfs-core`, `linfs-block` (WinDevice/ImageDevice + MBR/GPT/LUKS/LVM stubs + probe), `linfs-fs` (ext4/xfs/btrfs/f2fs), `linfs-mount` (WinFSP+fallback), `linfs-chroot` (Root translator), `linfs-terminal` (ConPTY+busybox+WSL), `linfs-cli` (`linfs list`), `linfs-gui`, `xtask`, `ui/`, `src/bin/linfs_loc_audit.rs`.

## Стан зараз

- **Band 200 DONE** — scaffold + block probe + `cargo test` pass + `cargo fmt` clean + git push to `platinoff/LinFS:main`.
- `cargo run -p linfs-cli -- list` enumerates `\\.\PhysicalDriveN` (stub) and probes GPT/MBR/LUKS/LVM; `linfs-block::probe::probe_fs` detects ext4/xfs/btrfs/f2fs magic.
- `cargo test` — all crates compile (windows `Win32_Security` feature, `linfs-block` stub win.rs); no ext4 driver yet.
- `PRODUCTS.md` registered in GSV (`docs/gsv/PRODUCTS.md` row `linfs`), `gsv.code-workspace` includes `../LinFS`, `docs/HANDOFF_NEW_SESSION.md` + `docs/NEXT_SESSION_PROMPT.md` present (this file).
- **Next = band 201** — ext4 read-only driver (`crates/linfs-fs/src/ext4/superblock.rs`, `group.rs`, `extent.rs`, `dir.rs` htree, `inode.rs`, `xattr.rs`) + fixtures `tests/fixtures/*.img` via `scripts/mkfixtures.sh` on Linux.

## Build/test (MSYS2 bash)

```bash
export PATH="/c/Users/${USER}/.cargo/bin:$HOME/.cargo/bin:/ucrt64/bin:/usr/bin:$PATH"
export RUSTUP_TOOLCHAIN="stable-x86_64-pc-windows-gnu"
cd /s/rust/LinFS
cargo fmt --all && cargo clippy --all-targets && cargo test && cargo run --bin linfs-loc-audit 2>&1 | head
cargo run -p linfs-cli -- list
```

Do not stage `data/*` (except `.gitkeep`), `.env*`, `*.pem`.

## Git

- `cargo xtask` в LinFS — `products|disk|loc-audit`; в GSV — повний `cargo xtask products` показує `linfs` як registered `rust` (`sibling`, `git=true`, `cargo=true`).
- Commit: `cargo fmt --all` → `cargo test` → `git add` (no `git add -A`) → `git commit -m "band 201: ..."` → `git push` (origin `platinoff/LinFS`).
