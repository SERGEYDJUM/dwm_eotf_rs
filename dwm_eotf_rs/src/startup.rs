use anyhow::{Context, Result, anyhow};
use std::process::Command;

const TASK_NAME: &str = "dwm_eotf_rs";

/// Returns the per-user task path: `\Users\<USERNAME>\`
fn task_path() -> String {
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "SYSTEM".to_string());
    format!(r"\Users\{}\", username)
}

/// Registers the app to run on Windows startup via Task Scheduler with highest
/// privileges (admin elevation). Uses PowerShell `Register-ScheduledTask` for
/// full control over task settings.
///
/// The task is created per-user under `\Users\<USERNAME>\` so that:
/// - It triggers only when the current user logs on
/// - The tray icon appears for the correct user session
///
/// Settings configured:
/// - **AllowStartIfOnBatteries** + **DontStopIfGoingOnBatteries**: runs on battery
/// - **ExecutionTimeLimit = PT0S**: no automatic stop after 3 days
/// - **StartWhenAvailable**: catch up if a trigger was missed
/// - **RestartCount / RestartInterval**: auto-retry on failure
/// - **RunLevel Highest**: `obtain_debug_privileges()` succeeds without UAC
pub fn register_startup(gamma: f32) -> Result<()> {
    let exe_path = std::env::current_exe().context("Failed to get current executable path")?;
    let exe_str = exe_path.display().to_string();

<<<<<<< HEAD
    let task_path = task_path();

    // Build a PowerShell script that mirrors the proven pattern:
    //   - New-ScheduledTaskAction with exe + arguments
    //   - New-ScheduledTaskTrigger -AtLogOn for this user only
    //   - New-ScheduledTaskSettingsSet with battery, restart, and availability flags
    //   - ExecutionTimeLimit = 'PT0S' (unlimited)
    //   - New-ScheduledTaskPrincipal with RunLevel Highest + Interactive logon
    let ps_script = format!(
        r#"
$action   = New-ScheduledTaskAction -Execute '{exe}' -Argument '{args}'
$trigger  = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -RestartCount 5 -RestartInterval (New-TimeSpan -Minutes 1)
$settings.ExecutionTimeLimit = 'PT0S'
$principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Highest

Register-ScheduledTask -TaskName '{name}' -TaskPath '{path}' -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Description 'Runs dwm_eotf_rs at user login' -Force
"#,
        exe = exe_str.replace('\'', "''"),
        args = format!("{:.3}", gamma),
        name = TASK_NAME,
        path = task_path,
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
=======
    let output = Command::new("schtasks")
        .args([
            "/Create", "/TN", TASK_NAME, "/TR", &command, "/SC", "ONLOGON", "/RL", "HIGHEST",
            "/F", // force overwrite if already exists
        ])
>>>>>>> 5e745ec6aad350e46f14575e47942bb6020fceb3
        .output()
        .context("Failed to run PowerShell for task registration")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "Register-ScheduledTask failed: {}",
            stderr.trim()
        ));
    }

    // Clean up any legacy entries from older versions
    let _ = cleanup_legacy_schtasks();
    let _ = cleanup_legacy_registry();

    Ok(())
}

/// Removes the app from Windows startup by deleting its scheduled task.
///
/// Silently succeeds if the task does not exist.
pub fn unregister_startup() -> Result<()> {

    let ps_script = format!(
        r#"Unregister-ScheduledTask -TaskName '{name}' -TaskPath '{path}' -Confirm:$false -ErrorAction SilentlyContinue"#,
        name = TASK_NAME,
        path = task_path(),
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .output()
        .context("Failed to run PowerShell for task removal")?;

    // PowerShell with SilentlyContinue won't error if the task doesn't exist,
    // but check anyway for unexpected failures
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
            eprintln!("Warning: Unregister-ScheduledTask stderr: {}", stderr.trim());
        }
    }

    // Also clean up legacy entries
    let _ = cleanup_legacy_schtasks();
    let _ = cleanup_legacy_registry();

    Ok(())
}

/// Checks whether the app is currently registered for Windows startup.
pub fn is_registered() -> bool {
    let ps_script = format!(
        r#"Get-ScheduledTask -TaskName '{name}' -TaskPath '{path}' -ErrorAction SilentlyContinue"#,
        name = TASK_NAME,
        path = task_path(),
    );

    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .output()
        .is_ok_and(|o| o.status.success() && !o.stdout.is_empty())
}

/// Removes a legacy task registered at the root `\dwm_eotf_rs` path by older versions
/// that used `schtasks.exe` directly.
fn cleanup_legacy_schtasks() -> Result<()> {
    let output = Command::new("schtasks")
        .args(["/Delete", "/TN", TASK_NAME, "/F"])
        .output()
        .context("Failed to run schtasks.exe for legacy cleanup")?;

    // Ignore all errors — the legacy task likely doesn't exist
    let _ = output;
    Ok(())
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
