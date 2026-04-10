use anyhow::Result;
use std::{mem::MaybeUninit, sync::mpsc};
use tracing::{debug, error, info};
use trayicon::*;
use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageA, GetMessageA, TranslateMessage};

use crate::{
    kill_dwm, patch_dwm,
    patcher::{SimplePatcher, build_aho_corasick},
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
    Exit,
}

pub fn run_in_tray(gamma: f32, skip_patching: bool, ignore_whitelist: bool) -> Result<()> {
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

    let mut initial_mode = Event::SetSRGB;
    let mut initial_icon = ICON_OFF;

    info!("Launching in Tray Mode...");

    if !skip_patching {
        info!("Patching DWM to use gamma {:.3}", gamma);

        match gamma {
            2.0 => {
                patch_dwm(&gamma20_patcher)?;
                initial_mode = Event::SetG20;
            }
            2.2 => {
                patch_dwm(&gamma22_patcher)?;
                initial_mode = Event::SetG22;
            }
            2.4 => {
                patch_dwm(&gamma24_patcher)?;
                initial_mode = Event::SetG24;
            }
            _ => {
                patch_dwm(&custom_patcher)?;
                initial_mode = Event::SetGCustom;
            }
        }

        initial_icon = ICON_ON;
    }

    let (tx, rx) = mpsc::channel::<Event>();

    let mut tray_icon = TrayIconBuilder::new()
        .sender(move |&e: &Event| {
            tx.send(e).ok();
        })
        .icon_from_buffer(initial_icon)
        .tooltip("dwm_eotf_rs")
        .on_right_click(Event::RightClick)
        .on_click(Event::LeftClick)
        .menu(build_menu(initial_mode, custom_gamma))
        .build()?;

    std::thread::spawn(move || {
        rx.iter().for_each(|m| {
            debug!("Processing event `{:?}`", m);

            let mut update_tray = |icon: &Icon| {
                tray_icon.set_menu(&build_menu(m, custom_gamma)).unwrap();
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
                    match kill_dwm() {
                        Ok(_) => update_tray(&icon_off),
                        Err(e) => error!("{}", e),
                    }
                }
                Event::SetG20 | Event::SetG22 | Event::SetG24 | Event::SetGCustom => {
                    info!("Patching DWM EOTF to use gamma {:.3}...", gamma_v);
                    match patch_dwm(patcher) {
                        Ok(_) => update_tray(&icon_on),
                        Err(e) => error!("{}", e),
                    }
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

fn build_menu(e: Event, custom_gamma: Option<f32>) -> MenuBuilder<Event> {
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
