use anyhow::{Context, Result, anyhow};
use std::process::{Command, Output};
use tracing::{info, warn};

/// Registers the app to run on Windows startup via Task Scheduler with highest
/// privileges (admin elevation). Uses PowerShell `Register-ScheduledTask` for
/// full control over task settings.
pub fn register_startup(gamma: f32) -> Result<()> {
    let exe_path = std::env::current_exe().context("Failed to get executable path")?;

    let script = format!(
        include_str!("../scripts/register_task.ps1"),
        INPUT_app_path = exe_path.to_string_lossy(),
        INPUT_app_args = format!("{:.3}", gamma)
    );

    let output = run_ps_script(&script).context("Failed to run task registration script")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Register-ScheduledTask failed: {}", stderr.trim()));
    }

    info!("Scheduled task with gamma {:.3}", gamma);
    Ok(())
}

/// Removes the app from Windows startup by deleting its scheduled task.
/// Silently succeeds if the task does not exist.
pub fn unregister_startup(all_users: bool) -> Result<()> {
    let script: &str;
    if !all_users {
        script = include_str!("../scripts/unregister_task.ps1");
    } else {
        script = include_str!("../scripts/unregister_all_tasks.ps1");
    }
    let output = run_ps_script(script).context("Failed to run task removal script")?;

    // PowerShell with SilentlyContinue won't error if the task doesn't exist,
    // but check anyway for unexpected failures
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
            warn!("Unregister-ScheduledTask stderr: {}", stderr.trim());
        }
    }

    info!(
        "Removed {} from scheduler",
        if !all_users {
            "task"
        } else {
            "task(s)"
        }
    );

    Ok(())
}

/// Checks whether the app is currently registered for Windows startup.
pub fn is_registered() -> bool {
    let script = include_str!("../scripts/check_registration.ps1");
    run_ps_script(script).is_ok_and(|o| o.status.success() && !o.stdout.is_empty())
}

fn run_ps_script(script: &str) -> std::io::Result<Output> {
    Command::new("powershell")
        .args(["-nop", "-noni", "-c", script])
        .output()
}
