mod args;
mod patcher;
mod startup;
mod tray;

use std::{path::Path, process::exit};

use anyhow::{Ok, Result, anyhow};
use clap::Parser as _;
use shader_patcher::{
    ShaderPatcher,
    winapi::{kill_process_by_name, obtain_debug_privileges},
};
use tracing::{debug, error, info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use windows::Win32::{
    System::Console::GetConsoleWindow,
    UI::WindowsAndMessaging::{SW_HIDE, ShowWindow},
};

use crate::{
    args::{Args, Commands},
    patcher::{SimplePatcher, build_aho_corasick},
    startup::{register_startup, unregister_startup},
    tray::run_in_tray,
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

    let args = Args::parse();

    debug!("{:?}", args);

    if let Err(err) = execute(&args) {
        error!("{}", err);
        exit(1);
    }
}

fn execute(args: &Args) -> Result<()> {
    if let Some(cmd) = &args.command {
        return match cmd {
            Commands::Restore => kill_dwm(),
            Commands::Schedule => register_startup(args.gamma),
            Commands::Unschedule => unregister_startup(false),
            Commands::UnscheduleAll => unregister_startup(true),
            Commands::Dump {
                big_shaders,
                output_dir,
            } => dump_shaders(output_dir, *big_shaders),
        };
    }

    if args.compatibility_mode {
        patch_dwm(&SimplePatcher::new(
            &build_aho_corasick()?,
            args.gamma,
            args.ignore_whitelist,
        ))
    } else {
        hide_cmd();
        run_in_tray(
            args.gamma,
            args.wait_time,
            args.skip_patching,
            args.ignore_whitelist,
        )
    }
}

fn hide_cmd() {
    let _ = unsafe { ShowWindow(GetConsoleWindow(), SW_HIDE) };
}

fn dump_shaders(path: &Path, only_big: bool) -> Result<()> {
    debug!("Obtaining debugging privileges...");
    obtain_debug_privileges()?;

    info!("Dumping shaders to `{}`...", path.to_string_lossy());
    let n = ShaderPatcher::open_restarted(DWM_EXE, DWM_DLL)?.execute_shader_dump(path, only_big)?;
    info!("{} shaders were dumped", n);
    Ok(())
}

fn kill_dwm() -> Result<()> {
    let pid = kill_process_by_name(DWM_EXE)?;
    info!("Killed `{}` process with PID {}", DWM_EXE, pid);
    Ok(())
}

fn patch_dwm(patcher: &SimplePatcher) -> Result<()> {
    debug!("Obtaining debugging privileges...");
    obtain_debug_privileges()?;

    match ShaderPatcher::open_restarted(DWM_EXE, DWM_DLL)?.execute_patching(patcher)? {
        0 => Err(anyhow!("No shaders were patched!")),
        _ => Ok(()),
    }
}
