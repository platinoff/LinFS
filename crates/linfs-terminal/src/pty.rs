use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// ConPTY wrapper — MVP uses std::process pipes; Windows ConPTY via
/// `CreatePseudoConsole` is stretch (band 208+). API is stable.
pub struct Pty {
    child: Child,
    stdout: std::process::ChildStdout,
}

impl Pty {
    /// Spawn a shell command with pseudo-terminal size (cols/rows recorded but not enforced in MVP).
    pub fn spawn(shell: &str, _cols: u16, _rows: u16) -> linfs_core::Result<Self> {
        // Use cmd.exe /C on Windows, sh -c elsewhere to match test `cmd.exe /c echo hi`
        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("cmd.exe");
            c.arg("/c").arg(shell);
            c
        } else {
            let mut c = Command::new("sh");
            c.arg("-c").arg(shell);
            c
        };
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // Hide console window on Windows
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = cmd.spawn().map_err(|e| {
            linfs_core::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("spawn {shell}: {e}"),
            ))
        })?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| linfs_core::Error::Corruption("spawn: no stdout".into()))?;
        Ok(Self { child, stdout })
    }

    pub fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if let Some(stdin) = self.child.stdin.as_mut() {
            stdin.write(data)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "no stdin",
            ))
        }
    }

    /// Read with timeout (MVP blocks up to timeout, then returns available).
    pub fn read_timeout(&mut self, timeout: Duration) -> linfs_core::Result<String> {
        use std::io::ErrorKind;
        // Set non-blocking via timeout thread? For MVP, wait for child with timeout
        let start = std::time::Instant::now();
        let mut buf = Vec::new();
        let mut tmp = [0u8; 1024];
        // Poll child exit + stdout
        loop {
            // Try read available without blocking forever: set read timeout via wait
            match self.stdout.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => {
                    buf.extend_from_slice(&tmp[..n]);
                    // If child has exited and drained, break
                    if let Ok(Some(_)) = self.child.try_wait() {
                        // drain remaining
                        let mut extra = Vec::new();
                        let _ = self.stdout.read_to_end(&mut extra);
                        buf.extend(extra);
                        break;
                    }
                    if start.elapsed() > timeout {
                        break;
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    if start.elapsed() > timeout {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => return Err(linfs_core::Error::Io(e)),
            }
            if start.elapsed() > timeout {
                break;
            }
            if buf.len() > 8192 {
                break;
            }
        }
        // Also wait for child to ensure output flushed
        let _ = self.child.wait();
        // Drain any remaining stdout
        let mut rest = Vec::new();
        let _ = self.stdout.read_to_end(&mut rest);
        buf.extend(rest);
        Ok(String::from_utf8_lossy(&buf).to_string())
    }

    pub fn resize(&self, _cols: u16, _rows: u16) -> linfs_core::Result<()> {
        // ConPTY resize via `SetConsoleScreenBufferSize` / `ResizePseudoConsole` — stretch
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pty_echo() {
        let shell = if cfg!(windows) { "echo hi" } else { "echo hi" };
        let mut pty = Pty::spawn(shell, 80, 24).expect("spawn");
        let out = pty.read_timeout(Duration::from_secs(2)).expect("read");
        assert!(out.contains("hi"), "expected hi in {out:?}");
    }
}
