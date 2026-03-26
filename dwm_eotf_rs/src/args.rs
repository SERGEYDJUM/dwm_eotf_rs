use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// The app will patch DWM and exit immidiately
    #[arg(short, long)]
    pub compatibility_mode: bool,

    /// The tray mode will not patch DWM at the start
    #[arg(short, long)]
    pub skip_patching: bool,

    /// Shader whitelist will not be used
    #[arg(short, long)]
    pub ignore_whitelist: bool,

    /// Restores original EOTF by restarting the DWM
    #[arg(short, long)]
    pub restore: bool,

    /// Gamma value (primary option in both modes)
    #[arg(default_value_t = 2.2)]
    pub gamma: f32,

    /// Dumps DWM's original shaders as DXBC and exits
    #[arg(short, long)]
    pub dump_shaders: bool,

    /// Prevents recursive dumping of sub-shaders
    #[arg(long)]
    pub big_shaders: bool,

    /// Target directory for dumped DXBC files
    #[arg(long, default_value = "shaders/dumped")]
    pub output_dir: PathBuf,
}
