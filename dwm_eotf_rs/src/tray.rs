use anyhow::Result;
use std::{mem::MaybeUninit, sync::mpsc, time::Duration};
use tracing::{debug, error, info};
use trayicon::*;
use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageA, GetMessageA, TranslateMessage};

use crate::{
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

pub fn run_in_tray(
    gamma: f32,
    wait_time: u64,
    skip_patching: bool,
    ignore_whitelist: bool,
) -> Result<()> {
    info!("Launching in Tray Mode...");

    let aho = build_aho_corasick()?;

    let is_startup_registered = startup::is_registered();

    let icon_off = Icon::from_buffer(ICON_OFF, None, None)?;
    let icon_on = Icon::from_buffer(ICON_ON, None, None)?;

    let initial_mode = Event::SetSRGB;
    let initial_icon = ICON_OFF;

    let custom_gamma = match gamma {
        2.0 | 2.2 | 2.4 => None,
        _ => Some(gamma),
    };

    let (tx, rx) = mpsc::channel::<Event>();

    // Clone tx before it gets moved into the tray builder's sender closure
    let tx_init = tx.clone();

    let mut tray_icon = TrayIconBuilder::new()
        .sender(move |&e: &Event| {
            tx.send(e).ok();
        })
        .icon_from_buffer(initial_icon)
        .tooltip("dwm_eotf_rs")
        .on_right_click(Event::RightClick)
        .on_click(Event::LeftClick)
        .menu(build_menu(
            initial_mode,
            custom_gamma,
            is_startup_registered,
        ))
        .build()?;

    // Spawn a deferred thread to send the initial patch event after DWM has time to start
    if !skip_patching {
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(wait_time));
            tx_init.send(Event::SetGamma(gamma)).ok();
        });
    }

    std::thread::spawn(move || {
        // Track the current gamma mode and startup state so we can update them
        let mut current_mode = initial_mode;
        let mut is_startup_registered = is_startup_registered;

        rx.iter().for_each(|e| {
            debug!("Processing event `{:?}`", e);

            let mut update_tray = |e: Event, startup_registered: bool, icon: &Icon| {
                tray_icon
                    .set_menu(&build_menu(e, custom_gamma, startup_registered))
                    .unwrap();
                tray_icon.set_icon(icon).unwrap();
            };

            match e {
                Event::SetSRGB => {
                    info!("Restoring DWM EOTF...");

                    match kill_dwm() {
                        Ok(_) => {
                            current_mode = e;
                            update_tray(e, is_startup_registered, &icon_off);
                        }
                        Err(err) => error!("{}", err),
                    }
                }
                Event::SetGamma(g) => {
                    info!("Patching DWM EOTF to use gamma {:.3}...", g);

                    match patch_dwm(&SimplePatcher::new(&aho, g, ignore_whitelist)) {
                        Ok(_) => {
                            current_mode = e;
                            update_tray(e, is_startup_registered, &icon_on);

                            // Auto-update task if startup is registered
                            if is_startup_registered {
                                if let Err(err) = startup::register_startup(g) {
                                    error!("Failed to update startup task: {}", err);
                                } else {
                                    debug!("Updated startup task to use gamma {:.3}", g);
                                }
                            }
                        }
                        Err(err) => error!("{}", err),
                    }
                }
                Event::ToggleStartup => {
                    if is_startup_registered {
                        match startup::unregister_startup() {
                            Ok(_) => is_startup_registered = false,
                            Err(e) => error!("Failed to remove startup registration: {}", e),
                        }
                    } else {
                        let g = if let Event::SetGamma(current_gamma) = current_mode {
                            current_gamma
                        } else {
                            gamma
                        };

                        match startup::register_startup(g) {
                            Ok(_) => is_startup_registered = true,
                            Err(err) => error!("Failed to register for startup: {}", err),
                        }
                    }

                    // Rebuild menu with the current mode icon
                    update_tray(
                        current_mode,
                        is_startup_registered,
                        if current_mode == Event::SetSRGB {
                            &icon_off
                        } else {
                            &icon_on
                        },
                    );
                }
                Event::RightClick | Event::LeftClick => {
                    tray_icon.show_menu().unwrap();
                }
                Event::Exit => {
                    info!("Shutting down...");
                    std::process::exit(0);
                }
            }
        })
    });

    win_main()
}

fn build_menu(e: Event, custom_gamma: Option<f32>, startup_registered: bool) -> MenuBuilder<Event> {
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
        .checkable("Autostart", startup_registered, Event::ToggleStartup)
        .separator()
        .item("Exit", Event::Exit)
}

fn win_main() -> Result<()> {
    let mut lpmsg = MaybeUninit::uninit();

    unsafe {
        while GetMessageA(lpmsg.as_mut_ptr(), None, 0, 0).0 > 0 {
            let _ = TranslateMessage(lpmsg.as_ptr());
            DispatchMessageA(lpmsg.as_ptr());
        }
    }

    Ok(())
}
