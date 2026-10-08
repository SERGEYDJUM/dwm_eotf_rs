use crate::registry;
use anyhow::Result;
use tracing::debug;

const DEFAULT_GAMMA: f32 = 2.2;
const DEFAULT_WAIT_TIME: f32 = 5.0;

#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Exponent to use during EOTF patching
    #[arg(default_value_t = DEFAULT_GAMMA)]
    pub gamma: f32,

    /// Patch DWM and exit (disables tray mode)
    #[arg(short, long)]
    pub compatibility_mode: bool,

    /// Optional Multi-Plane Overlay toggle (set to false to prevent some apps from bypassing DWM)
    #[arg(short, long)]
    pub mpo_state: Option<bool>,

    /// Optional target SDR brightness in nits (depends on display setup, see ReadMe)
    #[arg(short, long)]
    pub nits: Option<f32>,

    /// Patch every shader that contains sRGB EOTF or alpha correction patterns
    #[arg(short, long)]
    pub ignore_whitelist: bool,

    /// Prevent automatic patching on app start (tray mode only)
    #[arg(short, long)]
    pub skip_patching: bool,

    /// Delay (in seconds) before patching on start (tray mode only)
    #[arg(short, long, default_value_t = DEFAULT_WAIT_TIME)]
    pub wait_time: f32,

    /// Optional brightness multiplier (legacy dwm_eotf compatible factor)
    #[arg(long)]
    pub brightness: Option<f32>,

    /// Disable alpha correction fix when increasing brightness (see ReadMe)
    #[arg(long)]
    pub no_alpha_fix: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, clap::Subcommand)]
pub enum Commands {
    /// Dumps DWM's original shaders as DXBC
    Dump {
        /// Prevents recursive dumping of sub-shaders
        #[arg(short, long)]
        big_shaders: bool,

        /// Target directory for dumped DXBC files
        #[arg(short, long, default_value = "shaders/dumped")]
        output_dir: std::path::PathBuf,
    },

    /// Restores original sRGB EOTF and enables Multi-Plane Overlay
    Restore,

    /// Creates a task ('dwm_eotf_rs') that runs the app on user logon
    Schedule,

    /// Removes the startup task from Task Scheduler
    Unschedule {
        /// Remove the task for all users
        #[arg(short, long)]
        all: bool,
    },
}

impl Args {
    pub fn effective_brightness(&self) -> Result<f32> {
        if let Some(target_nits) = self.nits
            && let Some(reported_nits) = registry::get_primary_sdr_white_level()?
        {
            let factor = target_nits / reported_nits;
            debug!(
                "Reported SDR paper white level is {:.1} nits, using {:.3}x brightness factor",
                reported_nits, factor
            );
            return Ok(factor);
        }

        // dwm_eotf compatible multiplier
        Ok(self.brightness.unwrap_or(1.0).sqrt())
    }

    pub fn serialize_args(&self) -> String {
        let mut arguments = Vec::with_capacity(6);

        if self.ignore_whitelist {
            arguments.push("-i".to_string());
        }

        if self.skip_patching {
            arguments.push("-s".to_string());
        }

        if self.compatibility_mode {
            arguments.push("-c".to_string());
        }

        if let Some(mpo) = self.mpo_state {
            arguments.push(format!("-m {}", mpo));
        }

        if self.wait_time != DEFAULT_WAIT_TIME {
            arguments.push(format!("-w {:.1}", self.wait_time));
        }

        if let Some(nits) = self.nits {
            arguments.push(format!("-n {:.1}", nits));
        }

        if let Some(brightness) = self.brightness {
            arguments.push(format!("--brightness {:.3}", brightness));
        }

        if self.no_alpha_fix {
            arguments.push("--no-alpha-fix".to_string());
        }

        arguments.push(format!("{:.3}", self.gamma));
        arguments.join(" ")
    }
}
