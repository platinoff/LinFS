use std::process::Command;

/// Check if `wsl.exe` is available and reports status.
pub fn wsl_available() -> bool {
    Command::new("wsl")
        .arg("--status")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Build the `wsl chroot /mnt/linfs/<id> /bin/bash` command for Tier 2.
/// Caller mounts WinFSP drive as drvfs inside WSL first (`mount -t drvfs M: /mnt/linfs/id`).
pub fn wsl_chroot_command(mount_id: &str, cmd: &str) -> Vec<String> {
    vec![
        "wsl".to_string(),
        "bash".to_string(),
        "-c".to_string(),
        format!("chroot /mnt/linfs/{mount_id} {cmd}"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wsl_command_builds() {
        let v = wsl_chroot_command("abc", "/bin/bash");
        assert!(v.join(" ").contains("chroot /mnt/linfs/abc"));
    }
    #[test]
    fn wsl_available_does_not_panic() {
        let _ = wsl_available();
    }
}
