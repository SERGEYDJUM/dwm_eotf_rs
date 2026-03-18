// use std::{thread::sleep, time::Duration};

use dwm_eotf_rs::error::Result;
use dwm_eotf_rs::utils::{SHADER_HASHES, dump_shaders};
use dwm_eotf_rs::{DwmProcess, utils::grant_debug_privileges};
use tracing::debug;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    grant_debug_privileges()?;

    let dwm = DwmProcess::open_restarted()?;

    // dwm.suspend_process()?;
    // // dwm.dump_shaders()?;
    // dwm.resume_process()?;
    // // sleep(Duration::from_secs(3));
    // dwm.kill()?;

    // let mut rdata = File::open("dwmcore.rdata")?;
    // let mut rdata_bytes = vec![];
    // rdata.read_to_end(&mut rdata_bytes)?;

    dwm.suspend_process()?;

    let n_dumped = dump_shaders(dwm.dwmcore_read_memory()?.as_slice())?;

    debug!("Dumped {} shaders", n_dumped);

    dwm.resume_process()?;

    for digest in &SHADER_HASHES {
        let hash: u128 = *bytemuck::from_bytes(digest);
        let hash_rev: Vec<u8> = digest.iter().copied().rev().collect();
        let hash_rev: u128 = *bytemuck::from_bytes(&hash_rev);

        debug!("Stored hash: {:X}", hash);
        debug!("       rev:  {:X}", hash_rev);
    }

    Ok(())
}
