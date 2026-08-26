# LinFS Installer (band 212)

- Requires `winfsp.msi` (signed WinFSP driver) placed in `installer/` before building Inno Setup.
- Build: `iscc installer/LinFS.iss` -> `installer/Output/LinFS-1.0.0-x64.exe`
- Portable: `cargo build --release` -> `target/release/linfs.exe` (no installer needed, uses fallback axum browser on 127.0.0.1:9998 when WinFSP absent).

Fallback tested: `cargo run -p linfs-cli -- list` + `cargo run -p linfs-cli -- chroot <img> -- cat /etc/hostname`
