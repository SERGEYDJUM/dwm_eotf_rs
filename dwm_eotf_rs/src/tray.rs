use anyhow::Result;
use shader_patcher::winapi::obtain_debug_privileges;
use std::{mem::MaybeUninit, sync::mpsc};
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

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Event {
    RightClick,
    LeftClick,
    SetSRGB,
    SetG20,
    SetG22,
    SetG24,
    SetGCustom,
    ToggleStartup,
    Exit,
}

pub fn run_in_tray(gamma: f32, skip_patching: bool, ignore_whitelist: bool) -> Result<()> {
    obtain_debug_privileges()?;

    let aho = build_aho_corasick()?;

    let gamma20_patcher = SimplePatcher::new(aho.clone(), 2.0, ignore_whitelist)?;
    let gamma22_patcher = SimplePatcher::new(aho.clone(), 2.2, ignore_whitelist)?;
    let gamma24_patcher = SimplePatcher::new(aho.clone(), 2.4, ignore_whitelist)?;
    let custom_patcher = SimplePatcher::new(aho, gamma, ignore_whitelist)?;

    let icon_off = Icon::from_buffer(ICON_OFF, None, None)?;
    let icon_on = Icon::from_buffer(ICON_ON, None, None)?;

    let custom_gamma = match gamma {
        2.0 | 2.2 | 2.4 => None,
        _ => Some(gamma),
    };

    let initial_mode = Event::SetSRGB;
    let initial_icon = ICON_OFF;

    info!("Launching in Tray Mode...");

    let (tx, rx) = mpsc::channel::<Event>();

    // Clone tx before it gets moved into the tray builder's sender closure
    let tx_init = tx.clone();

    let startup_registered = startup::is_registered();

    let mut tray_icon = TrayIconBuilder::new()
        .sender(move |&e: &Event| {
            tx.send(e).ok();
        })
        .icon_from_buffer(initial_icon)
        .tooltip("dwm_eotf_rs")
        .on_right_click(Event::RightClick)
        .on_click(Event::LeftClick)
        .menu(build_menu(initial_mode, custom_gamma, startup_registered))
        .build()?;

    // Spawn a deferred thread to send the initial patch event after DWM has time to start
    if !skip_patching {
        let init_event = match gamma {
            2.0 => Event::SetG20,
            2.2 => Event::SetG22,
            2.4 => Event::SetG24,
            _ => Event::SetGCustom,
        };
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(5));
            tx_init.send(init_event).ok();
        });
    }

    std::thread::spawn(move || {
        // Track the current gamma mode and startup state so we can update them
        let mut current_mode = initial_mode;
        let mut is_startup_registered = startup_registered;

        rx.iter().for_each(|m| {
            debug!("Processing event `{:?}`", m);

            let mut update_tray = |mode: Event, startup_reg: bool, icon: &Icon| {
                tray_icon
                    .set_menu(&build_menu(mode, custom_gamma, startup_reg))
                    .unwrap();
                tray_icon.set_icon(icon).unwrap();
            };

            let (patcher, gamma_v) = match m {
                Event::SetG20 => (&gamma20_patcher, 2.0),
                Event::SetG22 => (&gamma22_patcher, 2.2),
                Event::SetG24 => (&gamma24_patcher, 2.4),
                _ => (&custom_patcher, gamma),
            };

            match m {
                Event::SetSRGB => {
                    info!("Restoring DWM EOTF...");
                    if let Err(e) = obtain_debug_privileges() {
                        error!("Failed to obtain debug privileges: {}", e);
                        return;
                    }
                    match kill_dwm() {
                        Ok(_) => {
                            current_mode = m;
                            update_tray(m, is_startup_registered, &icon_off);
                        }
                        Err(e) => error!("{}", e),
                    }
                }
                Event::SetG20 | Event::SetG22 | Event::SetG24 | Event::SetGCustom => {
                    info!("Patching DWM EOTF to use gamma {:.3}...", gamma_v);
                    if let Err(e) = obtain_debug_privileges() {
                        error!("Failed to obtain debug privileges: {}", e);
                        return;
                    }
                    match patch_dwm(patcher) {
                        Ok(_) => {
                            current_mode = m;
                            update_tray(m, is_startup_registered, &icon_on);

                            // Auto-update registry if startup is registered
                            if is_startup_registered {
                                if let Err(e) = startup::register_startup(gamma_v) {
                                    error!("Failed to update startup registration: {}", e);
                                } else {
                                    info!(
                                        "Updated startup registration to gamma {:.3}",
                                        gamma_v
                                    );
                                }
                            }
                        }
                        Err(e) => error!("{}", e),
                    }
                }
                Event::ToggleStartup => {
                    if is_startup_registered {
                        match startup::unregister_startup() {
                            Ok(_) => {
                                is_startup_registered = false;
                                info!("Removed from Windows startup");
                            }
                            Err(e) => error!("Failed to remove startup registration: {}", e),
                        }
                    } else {
                        // Determine the current gamma value for registration
                        let reg_gamma = match current_mode {
                            Event::SetG20 => 2.0,
                            Event::SetG22 => 2.2,
                            Event::SetG24 => 2.4,
                            _ => gamma,
                        };
                        match startup::register_startup(reg_gamma) {
                            Ok(_) => {
                                is_startup_registered = true;
                                info!(
                                    "Registered for Windows startup with gamma {:.3}",
                                    reg_gamma
                                );
                            }
                            Err(e) => error!("Failed to register for startup: {}", e),
                        }
                    }

                    // Rebuild menu with the current mode icon
                    let icon = if current_mode == Event::SetSRGB {
                        &icon_off
                    } else {
                        &icon_on
                    };
                    update_tray(current_mode, is_startup_registered, icon);
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

    if let Some(g) = custom_gamma {
        menu = menu
            .checkable(
                &format!("Custom Gamma ({:.3})", g),
                e == Event::SetGCustom,
                Event::SetGCustom,
            )
            .separator();
    }

    menu.checkable("Gamma 2.0", e == Event::SetG20, Event::SetG20)
        .checkable("Gamma 2.2", e == Event::SetG22, Event::SetG22)
        .checkable("Gamma 2.4", e == Event::SetG24, Event::SetG24)
        .separator()
        .checkable("Run on startup", startup_registered, Event::ToggleStartup)
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
