# LinFS Native TC GUI — Plan (OS Window, not Browser)

**User req:** No browser. Need **OS window** — cmd-like `mc` + graphical **Total Commander** dark theme, native Windows window.

**Goal:** Replace browser `axum` GUI with native `eframe/egui` Total Commander — dual-pane, F-keys, command line, dark theme, still 100% Rust.

**Stack:** `crates/linfs-gui` (`eframe 0.28` + `egui 0.28`, `tokio` for Fs, `linfs-fs` ext4, `linfs-chroot`, `linfs-terminal`), `ui/` kept as fallback but not primary. Rust 95–100% (`loc-audit --stretch-96`). MSYS2 bash `stable-x86_64-pc-windows-gnu`. Port 9998 only for fallback.

**Reference:** `docs/LINFS_SPEC.md` F5, `docs/LINFS_ARCHITECTURE.md`, previous browser plan `2026-08-27-linfs-perfect-gui.md` (now deprecated), `crates/linfs-gui/src/main.rs` stub.

---

## Tickets (GSV drain — one commit at end, but tickets tracked)

### TC-1: eframe scaffold + dark theme + Total Commander shell
**Files:** `crates/linfs-gui/Cargo.toml` (add `eframe`, `egui`), `crates/linfs-gui/src/main.rs` (eframe `TcApp`)
**Layout:**
- Native window 1280x720, dark theme `Visuals::dark()` + custom `TC_BLUE #0a3d62`, `TC_SEL #094771`
- Menu bar: File | Mark | Commands | Net | Show | Config | Help (egui `menu::bar`)
- Dual pane `Grid` 2 columns: left/right each `path bar` + `file table` + `status bar` (free, selected 0B/1.2M)
- Bottom: command line `>` (like mc) + function keys bar `F3 View  F4 Edit  F5 Copy  F6 Move  F7 Mkdir  F8 Delete  F10 Quit`
**Exit:** `cargo run -p linfs-gui` opens **OS window** (not browser), dark theme, two empty panes, F10 closes.

### TC-2: File tables + navigation + selection (mc/TC parity)
**Files:** `crates/linfs-gui/src/main.rs` (`TcPane` struct)
**Features:**
- `path bar` editable + breadcrumb `C:\` or `/` + `..` up
- `file table` `egui::Table` virtual for 100k: Name | Ext | Size | Date | Perms `rwxrwxrwx` | Owner
- Icons: `📁` dir, `📄` file, `🔗` symlink; dirs sorted first, then name
- Selection: `Ins` toggle, `*` wildcard, `+` select pattern, `Shift+Arrows` range, `Ctrl+A` all
- Active pane highlight `TC_SEL`, inactive `bg2`; `Tab` switch pane, `Enter` enter dir / view file, `Backspace` up
- Status: `3 / 42 files, 1.2M / 8G free` per pane
**Exit:** Synthetic demo ext4 `make_demo_image()` shows `/` with `etc`, `home`, `README.txt`, `etc/hostname` navigable via `Enter`.

### TC-3: Viewer / Editor / Hex / Permissions (F3/F4)
**Files:** `crates/linfs-gui/src/main.rs` (editor windows), `crates/linfs-fs` already
**Features:**
- `F3 View` → modal `egui::Window` with `ScrollArea` + `TextEdit` read-only (Monaco-like) for text files (`/etc/hostname` → `linfs-dev`), hex view `16 bytes per line + ascii` for binaries
- `F4 Edit` → editable `egui::TextEdit::multiline` + `Save` `Ctrl+S` → `fs.write_bytes(ino,0,content)` + `sync` + toast `Saved`
- `F7 Mkdir` → `egui::Window` `New folder:` + `fs.mkdir(pino,name)`
- `Shift+F4 New file` → `fs.create` + open editor
- `F8 Delete` → confirm `fs.unlink/rmdir` + toast
- `Alt+Enter Permissions` → `chmod` modal `0o644` input + `fs.chmod(ino,mode)` + `chown` stub
**Exit:** `F3` on `README.txt` shows `hello gui`, `F4` edit → `Save` → reopen shows edited, `F7` mkdir → `ls` shows new dir.

### TC-4: Copy/Move/Rename + Bulk + Progress + Mount manager
**Files:** `crates/linfs-gui/src/main.rs` (copy logic), `crates/linfs-block` `win::enumerate`
**Features:**
- `F5 Copy` → dialog `Copy 3 files to {other_pane_path}` + `Overwrite?` + progress `egui::ProgressBar` (for 1G folder via `fs.create/write` loop)
- `F6 Move` → `fs.rename` + progress
- `Shift+F5 Copy to` Windows host via `std::fs::write` (export), `Shift+F6` import via `FileDialog` (drag-drop host→LinFS via `fs.create/write`)
- `Mount manager` `Alt+F1`/`Alt+F2` drive selector: list `\\.\PhysicalDriveN` + GPT/LUKS/LVM + `*.img` `Attach` via `ImageDevice::open` → `Fs::open` replaces `AppState.fs`
- `Ctrl+R` refresh, `Ctrl+K` search filter per pane
**Exit:** `F5` copy `README.txt` left→right copies bytes correctly, `drag-drop` host file into pane creates LinFS file.

### TC-5: Chroot terminal (cmd-like) + WSL bridge
**Files:** `crates/linfs-gui/src/main.rs` (terminal pane), `crates/linfs-chroot/src/root.rs`, `crates/linfs-terminal/src/pty.rs`
**Features:**
- Bottom terminal `egui::TextEdit` history + `ConPTY` via `Pty::spawn` (busybox `sh` `ls -l`, `cat`, `echo`) — output in `egui::ScrollArea` 100k lines
- `Ctrl+Enter` insert selected filename into command line
- Chroot: `Root::new(fs)` + `resolve("/", cmd_arg)` demo + `wsl_available()` autodetect → if WSL present `wsl bash -c "chroot /mnt/..."` else busybox
- `F9` menu → `Commands → Chroot here`
**Exit:** Terminal `cat /etc/hostname` shows `linfs-dev` (edited value after `F4`), `..` at `/` clamped.

---

## Verification (GSV rules)

```bash
C:\msys64\usr\bin\bash.exe -lc 'cargo fmt --all'
C:\msys64\usr\bin\bash.exe -lc 'cargo clippy --all-targets -- -D warnings'
C:\msys64\usr\bin\bash.exe -lc 'cargo test -v'
C:\msys64\usr\bin\bash.exe -lc 'cargo run -p xtask -- loc-audit -- --stretch-96'
C:\msys64\usr\bin\bash.exe -lc 'cargo run -p linfs-gui -- --help 2>&1 | head'
# window: cargo run -p linfs-gui (opens OS window, not browser)
```

## Release

One commit `feat: native TC gui (OS window, Total Commander dark)` → `git push` → exe `target/debug/linfs-gui.exe` (OS window) + `target/release/linfs-gui.exe` + fallback `http://127.0.0.1:9998/` still available via `--browser` flag.
