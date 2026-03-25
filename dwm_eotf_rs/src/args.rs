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

    /// Restores original EOTF by restarting the DWM
    #[arg(short, long)]
    pub restore: bool,

    /// Gamma for compatibility mode
    #[arg(default_value_t = 2.2)]
    pub gamma: f32,
}
