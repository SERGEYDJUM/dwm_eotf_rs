use anyhow::Result;
use std::{mem::MaybeUninit, sync::mpsc, time::Duration};
use tracing::{debug, info};
use trayicon::*;
use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageA, GetMessageA, TranslateMessage};

use crate::{
    args::Args,
    kill_dwm, patch_dwm,
    patcher::{SimplePatcher, build_aho_corasick},
    startup,
};

static ICON_ON: &[u8] = include_bytes!("../icons/on.ico");
static ICON_OFF: &[u8] = include_bytes!("../icons/off.ico");

#[derive(Copy, Clone, PartialEq, Debug)]
enum Event {
    RightClick,
    LeftClick,
    SetSRGB,
    SetGamma(f32),
    ToggleStartup,
    Exit,
}

pub fn run_in_tray(mut args: Args) -> Result<()> {
    info!("Launching in Tray Mode...");

    let initial_registration = startup::is_registered()?;
    let initial_mode = Event::SetSRGB;
    let initial_icon = ICON_OFF;

    let custom_gamma = match args.gamma {
        2.0 | 2.2 | 2.4 => None,
        _ => Some(args.gamma),
    };

    let (tx, rx) = mpsc::channel::<Event>();

    if !args.skip_patching {
        tx.send(Event::SetGamma(args.gamma))?;
    }

    let mut tray_icon = TrayIconBuilder::new()
        .sender(move |&e: &Event| {
            tx.send(e).ok();
        })
        .icon_from_buffer(initial_icon)
        .tooltip("dwm_eotf_rs")
        .on_right_click(Event::RightClick)
        .on_click(Event::LeftClick)
        .menu(build_menu(initial_mode, custom_gamma, initial_registration))
        .build()?;

    let thread_jh = std::thread::spawn(move || -> Result<()> {
        let aho = build_aho_corasick()?;
        let icon_off = Icon::from_buffer(ICON_OFF, None, None)?;
        let icon_on = Icon::from_buffer(ICON_ON, None, None)?;

        let mut mode = initial_mode;
        let mut registration = initial_registration;

        macro_rules! update_tray {
            () => {
                tray_icon.set_menu(&build_menu(mode, custom_gamma, registration))?;
                tray_icon.set_icon(match mode {
                    Event::SetSRGB => &icon_off,
                    _ => &icon_on,
                })?;
            };
        }

        std::thread::sleep(Duration::from_secs(args.wait_time));

        for e in rx.iter() {
            debug!("Processing event `{:?}`", e);

            match e {
                Event::SetSRGB => {
                    info!("Restoring DWM EOTF...");
                    kill_dwm()?;
                    mode = e;
                    update_tray!();
                }
                Event::SetGamma(g) => {
                    info!("Patching DWM EOTF to use gamma {:.3}...", g);
                    patch_dwm(&SimplePatcher::new(&aho, g, args.ignore_whitelist))?;
                    (mode, args.gamma) = (e, g);

                    if registration {
                        debug!("Updating startup task...");
                        startup::register_startup(&args)?;
                    }

                    update_tray!();
                }
                Event::ToggleStartup => {
                    if registration {
                        info!("Removing startup task...");
                        startup::unregister_startup(false)?;
                    } else {
                        info!("Adding/updating startup task...");
                        startup::register_startup(&args)?;
                    }

                    registration = !registration;
                    update_tray!();
                }
                Event::RightClick | Event::LeftClick => tray_icon.show_menu()?,
                Event::Exit => break,
            }
        }

        Ok(())
    });

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

fn build_menu(e: Event, custom_gamma: Option<f32>, registration: bool) -> MenuBuilder<Event> {
    let mut menu = MenuBuilder::new()
        .checkable("sRGB (Disable)", e == Event::SetSRGB, Event::SetSRGB)
        .separator();

    if let Some(custom_gamma) = custom_gamma {
        menu = menu
            .checkable(
                &format!("Custom Gamma ({:.3})", custom_gamma),
                e == Event::SetGamma(custom_gamma),
                Event::SetGamma(custom_gamma),
            )
            .separator();
    }

    menu.checkable("Gamma 2.0", e == Event::SetGamma(2.0), Event::SetGamma(2.0))
        .checkable("Gamma 2.2", e == Event::SetGamma(2.2), Event::SetGamma(2.2))
        .checkable("Gamma 2.4", e == Event::SetGamma(2.4), Event::SetGamma(2.4))
        .separator()
        .checkable("Autostart", registration, Event::ToggleStartup)
        .separator()
        .item("Exit", Event::Exit)
}
