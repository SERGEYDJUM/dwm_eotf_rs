use anyhow::{Context, Result};
use winreg::enums::*;
use winreg::RegKey;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "dwm_eotf_rs";

/// Registers the app to run on Windows startup with the given gamma value.
///
/// Writes to `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` with the
/// current executable path and gamma argument. The exe path is quoted to
/// handle paths containing spaces.
pub fn register_startup(gamma: f32) -> Result<()> {
    let exe_path = std::env::current_exe().context("Failed to get current executable path")?;
    let command = format!("\"{}\" {:.3}", exe_path.display(), gamma);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (run_key, _) = hkcu
        .create_subkey(RUN_KEY)
        .context("Failed to open/create Run registry key")?;

    run_key
        .set_value(VALUE_NAME, &command)
        .context("Failed to write startup registry value")?;

    Ok(())
}

/// Removes the app from Windows startup by deleting its registry value.
///
/// Silently succeeds if the value does not exist.
pub fn unregister_startup() -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    if let Ok(run_key) = hkcu.open_subkey_with_flags(RUN_KEY, KEY_WRITE) {
        // delete_value returns an error if the value doesn't exist; ignore it
        let _ = run_key.delete_value(VALUE_NAME);
    }

    Ok(())
}

/// Checks whether the app is currently registered for Windows startup.
pub fn is_registered() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let Ok(run_key) = hkcu.open_subkey_with_flags(RUN_KEY, KEY_READ) else {
        return false;
    };

    run_key.get_value::<String, _>(VALUE_NAME).is_ok()
}
