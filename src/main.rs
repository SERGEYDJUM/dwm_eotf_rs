use dwm_eotf_rs::error::Result;
use dwm_eotf_rs::{DwmProcess, utils::grant_debug_privileges};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    grant_debug_privileges()?;

    let mut dwm = DwmProcess::open_restarted()?;
    
    dwm.suspend()?;
    dwm.read_ram()?;
    dwm.patch_shaders()?;
    dwm.commit_to_ram()?;
    dwm.resume()?;
    Ok(())
}
