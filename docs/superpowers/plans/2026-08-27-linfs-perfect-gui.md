# LinFS Perfect GUI — Plan (post-1.0)

**Goal:** Ship a *perfect* GUI for LinFS that makes Windows users feel at home browsing Linux filesystems — Explorer parity, Monaco editing, hex, bulk drag-drop, permissions, chroot terminal, mount manager.

**Stack:** `crates/linfs-gui` (axum 0.7 + tokio, serves `ui/` on `127.0.0.1:9998`, reuses `linfs-mount::fallback` + `linfs-chroot` + `linfs-fs` ext4), `ui/` thin JS (no Python/Java), Rust 95–100% (`loc-audit --stretch-96`).

**Reference:** `docs/LINFS_SPEC.md` F5/F6/F7, `docs/LINFS_ARCHITECTURE.md`, `ui/index.html` stub, `crates/linfs-gui/src/main.rs` stub.

---

## Tickets (to be drained in order, one commit per ticket is collapsed to one release commit here)

### GUI-1: axum server + /api/fs + static ui serve
**Files:** `crates/linfs-gui/Cargo.toml`, `crates/linfs-gui/src/main.rs`
**API:**
```
GET  /                    -> ui/index.html
GET  /style.css           -> ui/style.css
GET  /app.js              -> ui/app.js
GET  /api/fs/readdir?path=/etc
GET  /api/fs/stat?path=/etc/hostname
GET  /api/fs/read?path=/etc/hostname
POST /api/fs/write?path=/etc/hostname {content: base64}
POST /api/fs/mkdir {path}
POST /api/fs/unlink {path}
POST /api/fs/rename {from,to}
POST /api/fs/chmod {path,mode}
GET  /api/mount/list
POST /api/mount/attach {image}
```
**Exit:** `cargo run -p linfs-gui` → `http://127.0.0.1:9998/` serves HTML, `/api/fs/readdir?path=/` returns JSON.

### GUI-2: perfect ui/index.html + style.css
**Files:** `ui/index.html`, `ui/style.css`
**Layout:**
- Header: LinFS logo + mount selector + search + theme toggle
- Sidebar: device list (PhysicalDriveN, image) + partition tree
- Main: breadcrumb + file table (Name, Size, Perms `rwxrwxrwx`, Owner `uid:gid`, Mtime, Symlink→) + status bar (blocks free, journal OK)
- Modals: Monaco editor (text), hex editor (binary), permissions dialog, new file/folder, rename
- Terminal pane: xterm.js canvas + toolbar (clear, copy, font)
**Style:** VS Code dark theme parity, 12px Inter/Mono, 60fps virtual scroll for 100k entries, responsive 1280px+.
**Exit:** `ui/index.html` matches figma, no layout shift, Lighthouse 95+.

### GUI-3: ui/app.js (fetch tree, editor, hex, drag-drop, terminal, mount)
**Files:** `ui/app.js`
**Features:**
- `fetch(/api/fs/readdir)` → render tree with virtual scroll, click to navigate, breadcrumb
- Click file → `fetch(/api/fs/read)` → if text (utf8) open Monaco, else hex view (16 bytes per line + ascii)
- Edit → Monaco `Save` → `POST /api/fs/write` → toast `Saved` + status bar update
- Hex edit: click byte → edit → `Save` → write back
- Drag-drop: Windows → LinFS (via `fetch` + `FileReader` base64), LinFS → Windows (download)
- Permissions: right-click → `chmod` modal → `POST /api/fs/chmod`
- Mount manager: `GET /api/mount/list` → list `\\.\PhysicalDriveN` + GPT partitions, `Attach` image, `Mount` to `M:` or fallback
- Terminal: `xterm.js` over `POST /api/pty` (ConPTY via `linfs-terminal::Pty`)
- Search: `Ctrl+K` filter by name, `Ctrl+S` save, `F5` refresh
**Exit:** All flows work on synthetic `ext4-plain.img` without fixtures.

### GUI-4: linfs-gui mount manager + chroot terminal integration
**Files:** `crates/linfs-gui/src/main.rs` (mount endpoints), `crates/linfs-chroot/src/root.rs` reused, `crates/linfs-terminal/src/pty.rs` reused
**Exit:** `linfs chroot` via GUI button → `Root::resolve` demo + terminal `cat /etc/hostname` shows edited value.

---

## Verification (before commit)

```bash
C:\msys64\usr\bin\bash.exe -lc 'cargo fmt --all'
C:\msys64\usr\bin\bash.exe -lc 'cargo clippy --all-targets -- -D warnings'
C:\msys64\usr\bin\bash.exe -lc 'cargo test -v'
C:\msys64\usr\bin\bash.exe -lc 'cargo run -p xtask -- loc-audit -- --stretch-96'
C:\msys64\usr\bin\bash.exe -lc 'cargo run -p linfs-gui -- --help 2>&1 | head'
```

## Release

One commit `feat: perfect gui (band 212+ polish)` → `git push` → exe at `target/debug/linfs-gui.exe` / `target/release/linfs-gui.exe` + `cargo run -p linfs-gui` → `http://127.0.0.1:9998/`
