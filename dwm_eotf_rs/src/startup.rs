use anyhow::{Context, Result, anyhow};
use std::process::Command;

const TASK_NAME: &str = "dwm_eotf_rs";

/// Registers the app to run on Windows startup via Task Scheduler with highest
/// privileges (admin elevation). Uses `schtasks.exe` to create a task that
/// triggers at user logon.
///
/// The task runs with the `HIGHEST` run level so that `obtain_debug_privileges()`
/// succeeds without a UAC prompt at boot.
pub fn register_startup(gamma: f32) -> Result<()> {
    let exe_path = std::env::current_exe().context("Failed to get current executable path")?;
    let command = format!("\"{}\" {:.3}", exe_path.display(), gamma);

    let output = Command::new("schtasks")
        .args([
            "/Create", "/TN", TASK_NAME, "/TR", &command, "/SC", "ONLOGON", "/RL", "HIGHEST",
            "/F", // force overwrite if already exists
        ])
        .output()
        .context("Failed to run schtasks.exe")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("schtasks /Create failed: {}", stderr.trim()));
    }

    // Clean up any legacy registry Run key from older versions
    let _ = cleanup_legacy_registry();

    Ok(())
}

/// Removes the app from Windows startup by deleting its scheduled task.
///
/// Silently succeeds if the task does not exist.
pub fn unregister_startup() -> Result<()> {
    let output = Command::new("schtasks")
        .args(["/Delete", "/TN", TASK_NAME, "/F"])
        .output()
        .context("Failed to run schtasks.exe")?;

    // Ignore "task does not exist" errors
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // schtasks returns non-zero if the task doesn't exist — that's fine
        if !stderr.contains("does not exist") && !stderr.contains("not found") {
            return Err(anyhow!("schtasks /Delete failed: {}", stderr.trim()));
        }
    }

    // Clean up any legacy registry Run key from older versions
    let _ = cleanup_legacy_registry();

    Ok(())
}

/// Checks whether the app is currently registered for Windows startup.
pub fn is_registered() -> bool {
    Command::new("schtasks")
        .args(["/Query", "/TN", TASK_NAME])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Removes the legacy `HKCU\...\Run` registry entry left by older versions.
fn cleanup_legacy_registry() -> Result<()> {
    let output = Command::new("reg")
        .args([
            "delete",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            TASK_NAME,
            "/f",
        ])
        .output()
        .context("Failed to run reg.exe")?;

    // Ignore errors — the key might not exist
    let _ = output;
    Ok(())
}
