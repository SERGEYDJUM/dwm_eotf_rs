use std::{thread::sleep, time::Duration};

use tracing::Level;
use winsafe::SysResult;

use dwm_eotf_rs::{DwmProcess, utils::grant_debug_privileges};

fn main() -> SysResult<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    grant_debug_privileges()?;

    let dwm = DwmProcess::open()?;
    
    dwm.suspend_process()?;
    sleep(Duration::from_secs(3));
    dwm.resume_process()?;
    sleep(Duration::from_secs(3));
    dwm.kill()?;

    Ok(())
}
