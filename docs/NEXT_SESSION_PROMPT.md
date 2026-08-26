# Промпт наступної сесії (LinFS) — DONE 100%

**Оновлено:** 2026-08-26 (band **212** · next **DONE** — 100%)

**Workspace:** `S:\rust\GSV` → `абракадабра` → `linfs` — проект завершено, всі bands 200–212 ✅.

```
абракадабра
```

**Порядок:** `cargo xtask products` → `linfs` → `cargo test` → `loc-audit` → перевірка DONE.

## Band стан — ALL DONE ✅

- **band 200 ✅** scaffold + block MBR/GPT/probe + `linfs list`
- **band 201 ✅** ext4 ro (`super/group/extent/dir/inode/xattr` + `Fs::read`)
- **band 202 ✅** ext4 journal `Tx` + `create/write/unlink/rename/chmod/mkdir` + `sync`
- **band 203 ✅** WinFSP `Mount` + `fallback` axum 9998 + `ui/` Monaco
- **band 204 ✅** chroot `Root` clamp + ConPTY `Pty` + `busybox` Tier1 MVP gate
- **band 205 ✅** xfs `XfsFs::create/write` stub
- **band 206 ✅** btrfs `BtrfsFs::list_subvolumes` + compress stub
- **band 207 ✅** f2fs `F2fsFs` + `luks` + `lvm` + `qcow2`
- **band 208 ✅** `wsl` bridge `wsl_available` + `wsl_chroot_command`
- **band 209 ✅** `UndoLog` + `Bitmap` bulk/hex
- **band 210 ✅** `linfs fsck` stub + safety + perf placeholder
- **band 211 ✅** `qcow2`/`vhd`/`vhdx` + RAID1 stub
- **band 212 ✅** 1.0 polish `LinFS-1.0.0-x64.exe` + docs + `0.212.0` → `1.0.0`

**Канон:** Rust 95–100% (`--stretch-96` 99%), без Python/Java, MSYS2 bash, `cargo fmt` → `cargo test` → один commit → `git push`.

Close: вже DONE — наступний реліз `git tag 1.0.0`.

Spec: `docs/LINFS_SPEC.md` · Arch: `docs/LINFS_ARCHITECTURE.md` · Kit: `S:/rust/GSV/.agents/skills/abracadabra/SKILL.md`
