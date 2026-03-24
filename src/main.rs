use dwm_eotf_rs::{TargetProcess, error::Result, patcher::HardCodedPatcher, winapi::grant_debug_privileges};

const DWM_EXE: &str = "dwm.exe";
const DWM_DLL: &str = "dwmcore.dll";

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    grant_debug_privileges()?;

    let patcher = HardCodedPatcher::default();
    let mut dwm = TargetProcess::open_restarted(DWM_EXE, DWM_DLL)?;

    dwm.suspend()?;
    dwm.read_ram()?;

    if dwm.patch_shaders(&patcher)? != 0 {
        dwm.commit_to_ram()?;
        dwm.resume()?;
        Ok(())
    } else {
        dwm.resume()?;
        std::process::exit(1)
    }
}
