// use std::{thread::sleep, time::Duration};

use dwm_eotf_rs::error::Result;
use dwm_eotf_rs::{DwmProcess, utils::grant_debug_privileges};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    grant_debug_privileges()?;

    let dwm = DwmProcess::open()?;

    dwm.suspend_process()?;
    dwm.dump_shaders()?;
    dwm.resume_process()?;
    // sleep(Duration::from_secs(3));
    // dwm.kill()?;

    Ok(())
}
