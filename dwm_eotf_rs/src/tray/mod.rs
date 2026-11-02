mod listener;

use aho_corasick::AhoCorasick;
use anyhow::Result;
use std::{
    mem::MaybeUninit,
    sync::mpsc::{self, Receiver},
    time::Duration,
};
use tracing::{debug, info, warn};
use trayicon::*;
use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageA, GetMessageA, TranslateMessage};

use crate::{
    args::Args,
    get_brightness_factor, kill_dwm, patch_dwm,
    patcher::{SimplePatcher, build_aho_corasick},
    registry, startup,
    tray::listener::run_watcher_window,
    winapi,
};

static ICON_ON: &[u8] = include_bytes!("../../icons/on.ico");
static ICON_OFF: &[u8] = include_bytes!("../../icons/off.ico");

#[derive(Copy, Clone, PartialEq, Debug)]
enum Event {
    RightClick,
    LeftClick,
    SetSRGB,
    SetGamma(f32),
    ToggleStartup,
    ToggleMpo,
    Exit,
    DisplayChange,
}

#[derive(Debug)]
struct TrayState {
    mode: Event,
    mpo: bool,
    reg: bool,
    sdr: bool,
    args: Args,
}

impl TrayState {
    pub fn build_menu(&self) -> MenuBuilder<Event> {
        let e = self.mode;
        let mut menu = MenuBuilder::new()
            .checkable("sRGB (Disable)", e == Event::SetSRGB, Event::SetSRGB)
            .separator();

        match self.args.gamma {
            2.0 | 2.2 | 2.4 => (),
            _ => {
                menu = menu
                    .checkable(
                        &format!("Custom Gamma ({:.3})", self.args.gamma),
                        e == Event::SetGamma(self.args.gamma),
                        Event::SetGamma(self.args.gamma),
                    )
                    .separator();
            }
        };

        menu.checkable("Gamma 2.0", e == Event::SetGamma(2.0), Event::SetGamma(2.0))
            .checkable("Gamma 2.2", e == Event::SetGamma(2.2), Event::SetGamma(2.2))
            .checkable("Gamma 2.4", e == Event::SetGamma(2.4), Event::SetGamma(2.4))
            .separator()
            .checkable("Multi-Plane Overlay", self.mpo, Event::ToggleMpo)
            .separator()
            .checkable("Autostart", self.reg, Event::ToggleStartup)
            .separator()
            .item("Exit", Event::Exit)
    }
}

pub struct TrayController {
    state: TrayState,
    tray_icon: TrayIcon<Event>,
    rx: Receiver<Event>,
    aho: AhoCorasick,
    icon_off: Icon,
    icon_on: Icon,
}

impl TrayController {
    fn update_view(&mut self) -> Result<()> {
        let icon_variant = if self.state.sdr || self.state.mode == Event::SetSRGB {
            &self.icon_off
        } else {
            &self.icon_on
        };

        self.tray_icon.set_menu(&self.state.build_menu())?;
        self.tray_icon.set_icon(icon_variant)?;
        Ok(())
    }

    fn on_set_srgb(&mut self) -> Result<()> {
        info!("Restoring DWM EOTF...");
        kill_dwm()?;
        self.state.mode = Event::SetSRGB;
        Ok(())
    }

    fn on_set_gamma(&mut self, gamma: f32) -> Result<()> {
        let old_gamma = self.state.args.gamma;
        let brightness = get_brightness_factor(self.state.args.nits, self.state.args.brightness)?;

        if !self.state.sdr {
            info!(
                "Patching DWM EOTF to use gamma {:.3} and brightness factor {:.3}...",
                gamma, brightness
            );

            patch_dwm(&SimplePatcher::new(
                &self.aho,
                gamma,
                brightness,
                self.state.args.ignore_whitelist,
                self.state.args.no_alpha_fix,
            ))?;
        } else {
            warn!("HDR is disabled, DWM will not be patched!");
            self.on_set_srgb()?;
        }

        (self.state.mode, self.state.args.gamma) = (Event::SetGamma(gamma), gamma);

        if self.state.reg && old_gamma != gamma {
            debug!("Updating startup task...");
            startup::register_startup(&self.state.args)?;
        }

        Ok(())
    }

    fn on_toggle_startup(&mut self) -> Result<()> {
        if self.state.reg {
            info!("Removing startup task...");
            startup::unregister_startup(false)?;
        } else {
            info!("Scheduling startup task...");
            startup::register_startup(&self.state.args)?;
        }

        self.state.reg = !self.state.reg;
        Ok(())
    }

    fn on_toggle_mpo(&mut self) -> Result<()> {
        info!(
            "{} Multi-Plane Overlay (MPO)",
            if self.state.mpo {
                "Disabling"
            } else {
                "Enabling"
            }
        );

        registry::set_mpo_state(!self.state.mpo)?;
        self.state.mpo = !self.state.mpo;

        match self.state.mode {
            Event::SetGamma(gamma) => self.on_set_gamma(gamma),
            _ => self.on_set_srgb(),
        }?;

        Ok(())
    }

    fn on_display_change(&mut self) -> Result<()> {
        let sdr_old = self.state.sdr;
        self.state.sdr = !winapi::is_hdr_active()?;

        if self.state.sdr != sdr_old {
            debug!("HDR was {}", if sdr_old { "enabled" } else { "disabled" });

            if let Event::SetGamma(gamma) = self.state.mode {
                self.on_set_gamma(gamma)?;
            }
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        std::thread::sleep(Duration::from_secs_f32(self.state.args.wait_time));
        let mut maybe_event = self.rx.recv().ok();

        while let Some(event) = maybe_event {
            debug!("Processing event `{:?}`", event);

            match event {
                Event::SetSRGB => self.on_set_srgb()?,
                Event::SetGamma(g) => self.on_set_gamma(g)?,
                Event::ToggleStartup => self.on_toggle_startup()?,
                Event::ToggleMpo => self.on_toggle_mpo()?,
                Event::RightClick | Event::LeftClick => self.tray_icon.show_menu()?,
                Event::DisplayChange => self.on_display_change()?,
                Event::Exit => break,
            }

            self.update_view()?;
            maybe_event = self.rx.recv().ok();
        }

        Ok(())
    }
}

pub fn run_in_tray(args: Args) -> Result<()> {
    info!("Launching in Tray Mode...");

    let (tx, rx) = mpsc::channel::<Event>();
    let watcher_tx = tx.clone();

    if !args.skip_patching {
        tx.send(Event::SetGamma(args.gamma))?;
    }

    // Order makes sure initial SetGamma gets processed before any DisplayChange
    std::thread::spawn(move || run_watcher_window(watcher_tx));

    let tray_state = TrayState {
        mode: Event::SetSRGB,
        mpo: registry::is_mpo_enabled()?,
        reg: startup::is_registered()?,
        sdr: !winapi::is_hdr_active()?,
        args,
    };

    let tray_icon = TrayIconBuilder::new()
        .sender(move |&e: &Event| {
            tx.send(e).ok();
        })
        .icon_from_buffer(ICON_OFF)
        .tooltip("dwm_eotf_rs")
        .on_right_click(Event::RightClick)
        .on_click(Event::LeftClick)
        .menu(tray_state.build_menu())
        .build()?;

    let mut tray_controller = TrayController {
        state: tray_state,
        tray_icon,
        rx,
        aho: build_aho_corasick()?,
        icon_off: Icon::from_buffer(ICON_OFF, None, None)?,
        icon_on: Icon::from_buffer(ICON_ON, None, None)?,
    };

    let thread_jh = std::thread::spawn(move || tray_controller.run());

    let mut lpmsg = MaybeUninit::uninit();

    unsafe {
        while GetMessageA(lpmsg.as_mut_ptr(), None, 0, 0).0 > 0 {
            let _ = TranslateMessage(lpmsg.as_ptr());
            DispatchMessageA(lpmsg.as_ptr());

            if thread_jh.is_finished() {
                break;
            }
        }
    }

    thread_jh.join().expect("failed to join tray thread")
}
