use anyhow::Result;
use windows_registry::LOCAL_MACHINE;

const MPO_REG_KEY: &str = "SOFTWARE\\Microsoft\\Windows\\Dwm";
const MPO_REG_NAME: &str = "OverlayTestMode";
const MONITOR_DATA_STORE: &str =
    "SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers\\MonitorDataStore";

pub fn is_mpo_enabled() -> Result<bool> {
    let value = LOCAL_MACHINE
        .create(MPO_REG_KEY)?
        .get_u32(MPO_REG_NAME)
        .unwrap_or(0);

    Ok(value & 5 == 0)
}

pub fn set_mpo_state(enabled: bool) -> Result<()> {
    let value = if enabled { 0 } else { 5 };

    Ok(LOCAL_MACHINE
        .create(MPO_REG_KEY)?
        .set_u32(MPO_REG_NAME, value)?)
}

pub fn get_primary_sdr_white_level() -> Result<Option<f32>> {
    let key = LOCAL_MACHINE.open(MONITOR_DATA_STORE)?;

    let mut highest_level = 0u32;
    if let Ok(keys) = key.keys() {
        for subkey_name in keys {
            if let Ok(subkey) = key.open(&subkey_name)
                && let Ok(val) = subkey.get_u32("SDRWhiteLevel")
                && val > highest_level
            {
                highest_level = val;
            }
        }
    }

    if highest_level == 0 {
        return Ok(None);
    }

    // SDRWhiteLevel is stored in units of 1/1000 of 80 nits
    // e.g. 6000 * 80 / 1000 = 480 nits
    Ok(Some((highest_level as f32) * 80.0 / 1000.0))
}
