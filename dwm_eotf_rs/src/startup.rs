use anyhow::{Result, anyhow};
use std::process::{Command, Output};
use tracing::debug;

use crate::args::Args;

/// Creates or overwrites the startup task in Task Scheduler.
pub fn register_startup(args: &Args) -> Result<()> {
    let script = format!(
        include_str!("../scripts/register_task.ps1"),
        INPUT_app_path = std::env::current_exe()?.to_string_lossy(),
        INPUT_app_args = args.serialize_args()
    );

    let output = run_ps_script(&script)?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to register the task:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Deletes the startup task from Task Scheduler.
/// If `all_users` is false, a user-specific path will be used.
pub fn unregister_startup(all_users: bool) -> Result<()> {
    let script = if !all_users {
        include_str!("../scripts/unregister_task.ps1")
    } else {
        include_str!("../scripts/unregister_all_tasks.ps1")
    };

    let output = run_ps_script(script)?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to unregister the task:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Checks whether the user-specific startup task exists in Task Scheduler.
pub fn is_registered() -> Result<bool> {
    let script = include_str!("../scripts/check_registration.ps1");
    Ok(run_ps_script(script)?.status.success())
}

fn run_ps_script(script: &str) -> std::io::Result<Output> {
    debug!("Running PowerShell script:\n{}", script);

    Command::new("powershell")
        .args(["-nop", "-noni", "-c", script])
        .output()
}
