const DEFAULT_GAMMA: f32 = 2.2;
const DEFAULT_BRIGHTNESS: f32 = 1.0;
const DEFAULT_WAIT_TIME: f32 = 5.0;

#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Exponent to use during EOTF patching
    #[arg(default_value_t = DEFAULT_GAMMA)]
    pub gamma: f32,

    /// Patches DWM and exits (disables tray mode)
    #[arg(short, long)]
    pub compatibility_mode: bool,

    /// Prevents automatic patching on app start (tray mode)
    #[arg(short, long)]
    pub skip_patching: bool,

    /// Delay (in seconds) before automatic patching on app start (tray mode)
    #[arg(short, long, default_value_t = DEFAULT_WAIT_TIME)]
    pub wait_time: f32,

    /// Patch every shader that contains sRGB EOTF patterns
    #[arg(short, long)]
    pub ignore_whitelist: bool,

    /// Brightness multiplier (factor for nits)
    #[arg(long, default_value_t = DEFAULT_BRIGHTNESS)]
    pub brightness: f32,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, clap::Subcommand)]
pub enum Commands {
    /// Restores original sRGB EOTF (by restarting DWM)
    Restore,

    /// Creates a task ('dwm_eotf_rs') that runs the app on user logon
    Schedule,

    /// Removes the startup task ('dwm_eotf_rs') from Task Scheduler
    Unschedule,

    /// Removes the startup task ('dwm_eotf_rs') from Task Scheduler for all users
    UnscheduleAll,

    /// Dumps DWM's original shaders as DXBC
    Dump {
        /// Prevents recursive dumping of sub-shaders
        #[arg(short, long)]
        big_shaders: bool,

        /// Target directory for dumped DXBC files
        #[arg(short, long, default_value = "shaders/dumped")]
        output_dir: std::path::PathBuf,
    },
}

impl Args {
    pub fn serialize_args(&self) -> String {
        let mut arguments = Vec::with_capacity(5);

        if self.ignore_whitelist {
            arguments.push("-i".to_string());
        }

        if self.skip_patching {
            arguments.push("-s".to_string());
        }

        if self.compatibility_mode {
            arguments.push("-c".to_string());
        }

        if self.wait_time != DEFAULT_WAIT_TIME {
            arguments.push(format!("-w {}", self.wait_time));
        }

        if self.brightness != DEFAULT_BRIGHTNESS {
            arguments.push(format!("--brightness {}", self.wait_time));
        }

        if self.gamma != DEFAULT_GAMMA {
            arguments.push(format!("{:.3}", self.gamma));
        }

        arguments.join(" ")
    }
}
