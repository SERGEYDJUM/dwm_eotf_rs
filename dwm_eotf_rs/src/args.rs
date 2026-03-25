use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// The app will run in system tray until closed
    #[arg(short, long)]
    pub tray_mode: bool,

    /// The tray mode will patch DWM at the start
    #[arg(short, long)]
    pub patch_immidiately: bool,

    /// Restores original EOTF by restarting the DWM
    #[arg(short, long)]
    pub restore: bool,

    /// Gamma to use in patched EOTF
    #[arg(default_value_t = 2.2)]
    pub gamma: f32,
}
