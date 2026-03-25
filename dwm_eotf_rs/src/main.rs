mod args;
mod patcher;
mod tray;

use std::process::exit;

use anyhow::{Result, anyhow};
use clap::Parser;
use shader_patcher::{
    ShaderPatcher,
    winapi::{grant_debug_privileges, kill_process_by_name},
};
use tracing::{debug, error, info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::{args::Args, patcher::HardCodedPatcher, tray::run_tray};

const DWM_EXE: &str = "dwm.exe";
const DWM_DLL: &str = "dwmcore.dll";

fn patch_dwm(patcher: &HardCodedPatcher) -> Result<()> {
    let mut dwm = ShaderPatcher::open_restarted(DWM_EXE, DWM_DLL)?;

    dwm.suspend()?;
    dwm.read_ram()?;

    if dwm.patch_shaders(patcher)? != 0 {
        dwm.commit_to_ram()?;
        dwm.resume()?;
        Ok(())
    } else {
        dwm.resume()?;
        Err(anyhow!("No shaders were patched!"))
    }
}

fn kill_dwm() -> Result<()> {
    let pid = kill_process_by_name(DWM_EXE)?;
    info!("Killed `{}` process with PID {}", DWM_EXE, pid);
    Ok(())
}

fn execute(args: Args) -> Result<()> {
    debug!("{:?}", args);

    debug!("Granting debugging privileges...");
    grant_debug_privileges()?;

    if args.tray_mode {
        if args.restore {
            kill_dwm()?;
        }

        let gamma = if args.patch_immidiately {
            Some(args.gamma)
        } else {
            None
        };

        return run_tray(gamma);
    }

    if args.restore {
        return kill_dwm();
    }

    patch_dwm(&HardCodedPatcher::from_gamma(args.gamma)?)
}

fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    if let Err(err) = execute(Args::parse()) {
        error!("{}", err);
        exit(1);
    }
}
