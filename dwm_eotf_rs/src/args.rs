#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Exponent to use during EOTF patching
    #[arg(default_value_t = 2.2)]
    pub gamma: f32,

    /// Patches DWM and exits (disables tray mode)
    #[arg(short, long)]
    pub compatibility_mode: bool,

    /// Prevents automatic patching on app start (tray mode)
    #[arg(short, long)]
    pub skip_patching: bool,

    /// Delay (in seconds) before automatic patching on app start (tray mode)
    #[arg(short, long, default_value_t = 5)]
    pub wait_time: u64,

    /// Patch every shader with matching patterns
    #[arg(short, long)]
    pub ignore_whitelist: bool,

    /// Restores original sRGB EOTF (by restarting DWM) and exits
    #[arg(short, long)]
    pub restore: bool,

    /// Dumps DWM's original shaders as DXBC and exits
    #[arg(long)]
    pub dump_shaders: bool,

    /// Prevents recursive dumping of sub-shaders
    #[arg(long)]
    pub big_shaders: bool,

    /// Target directory for dumped DXBC files
    #[arg(long, default_value = "shaders/dumped")]
    pub output_dir: std::path::PathBuf,

    /// Registers the app to run on Windows startup and exits
    #[arg(long)]
    pub startup: bool,

    /// Removes the app from Windows startup and exits
    #[arg(long)]
    pub no_startup: bool,
}
