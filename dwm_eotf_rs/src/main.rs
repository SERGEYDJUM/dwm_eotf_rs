mod args;
mod patcher;
mod tray;

use std::process::exit;

use anyhow::{Result, anyhow};
use clap::Parser;
use shader_patcher::{
    ShaderPatcher,
    winapi::{obtain_debug_privileges, kill_process_by_name},
};
use tracing::{debug, error, info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::{
    args::Args,
    patcher::{SimplePatcher, build_aho_corasick},
    tray::run_tray,
};

const DWM_EXE: &str = "dwm.exe";
const DWM_DLL: &str = "dwmcore.dll";

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

fn execute(args: Args) -> Result<()> {
    debug!("{:?}", args);
    debug!("Obtaining debugging privileges...");
    obtain_debug_privileges()?;

    if args.dump_shaders {
        return dump_shaders(&args);
    }

    if !args.compatibility_mode {
        return run_tray(&args);
    }

    if args.restore {
        return kill_dwm();
    }

    patch_dwm(&SimplePatcher::new(
        build_aho_corasick()?,
        args.gamma,
        args.ignore_whitelist,
    )?)
}

fn patch_dwm(patcher: &SimplePatcher) -> Result<()> {
    if ShaderPatcher::open_restarted(DWM_EXE, DWM_DLL)?.execute_patching(patcher)? == 0 {
        return Err(anyhow!("No shaders were patched!"));
    }
    Ok(())
}

fn dump_shaders(args: &Args) -> Result<()> {
    info!(
        "Dumping shaders to `{}`...",
        args.output_dir.to_string_lossy()
    );

    let n_shaders = ShaderPatcher::open_restarted(DWM_EXE, DWM_DLL)?
        .execute_shader_dump(&args.output_dir, args.big_shaders)?;

    info!("{} shaders were dumped", n_shaders);
    Ok(())
}

fn kill_dwm() -> Result<()> {
    let pid = kill_process_by_name(DWM_EXE)?;
    info!("Killed `{}` process with PID {}", DWM_EXE, pid);
    Ok(())
}
