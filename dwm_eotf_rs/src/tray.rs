use core::mem::MaybeUninit;
use std::sync::mpsc;
use tracing::{info, warn};
use trayicon::*;
use winapi::um::{
    wincon::GetConsoleWindow,
    winuser::{self, SW_HIDE, ShowWindow},
};

use crate::{kill_dwm, patch_dwm, patcher::HardCodedPatcher};

static ICON_SRGB: &[u8] = include_bytes!("../icons/off.ico");
static ICON_G20: &[u8] = include_bytes!("../icons/g20.ico");
static ICON_G22: &[u8] = include_bytes!("../icons/g22.ico");
static ICON_G24: &[u8] = include_bytes!("../icons/g24.ico");

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Event {
    RightClick,
    LeftClick,
    SetSRGB,
    SetG20,
    SetG22,
    SetG24,
    Exit,
}

pub fn run_tray(gamma: Option<f32>) -> anyhow::Result<()> {
    let gamma20_patcher = HardCodedPatcher::from_gamma(2.0)?;
    let gamma22_patcher = HardCodedPatcher::from_gamma(2.2)?;
    let gamma24_patcher = HardCodedPatcher::from_gamma(2.4)?;

    let icon_srgb = Icon::from_buffer(ICON_SRGB, None, None)?;
    let icon_g20 = Icon::from_buffer(ICON_G20, None, None)?;
    let icon_g22 = Icon::from_buffer(ICON_G22, None, None)?;
    let icon_g24 = Icon::from_buffer(ICON_G24, None, None)?;

    let mut initial_mode = Event::SetSRGB;
    let mut initial_icon = ICON_SRGB;

    if let Some(gamma) = gamma {
        info!("Launching Tray Mode with initial gamma {}", gamma);

        match gamma {
            2.0 => {
                patch_dwm(&gamma20_patcher)?;
                initial_mode = Event::SetG20;
                initial_icon = ICON_G20;
            }
            2.2 => {
                patch_dwm(&gamma22_patcher)?;
                initial_mode = Event::SetG22;
                initial_icon = ICON_G22;
            }
            2.4 => {
                patch_dwm(&gamma24_patcher)?;
                initial_mode = Event::SetG24;
                initial_icon = ICON_G24;
            }
            _ => warn!("Tray Mode doesn't support custom gamma values!"),
        }
    } else {
        info!("Launched Tray Mode without patching")
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
        .menu(build_menu(initial_mode))
        .build()?;

    std::thread::spawn(move || {
        rx.iter().for_each(|m| match m {
            Event::SetSRGB => {
                info!("Restoring DWM EOTF...");
                kill_dwm().unwrap();
                tray_icon.set_menu(&build_menu(m)).unwrap();
                tray_icon.set_icon(&icon_srgb).unwrap();
            }
            Event::SetG20 => {
                info!("Setting gamma to 2.0...");
                patch_dwm(&gamma20_patcher).unwrap();
                tray_icon.set_menu(&build_menu(m)).unwrap();
                tray_icon.set_icon(&icon_g20).unwrap();
            }
            Event::SetG22 => {
                info!("Setting gamma to 2.2...");
                patch_dwm(&gamma22_patcher).unwrap();
                tray_icon.set_menu(&build_menu(m)).unwrap();
                tray_icon.set_icon(&icon_g22).unwrap();
            }
            Event::SetG24 => {
                info!("Setting gamma to 2.4...");
                patch_dwm(&gamma24_patcher).unwrap();
                tray_icon.set_menu(&build_menu(m)).unwrap();
                tray_icon.set_icon(&icon_g24).unwrap();
            }
            Event::RightClick | Event::LeftClick => {
                tray_icon.show_menu().unwrap();
            }
            Event::Exit => {
                info!("Shutting down Tray Mode...");
                std::process::exit(0);
            }
        })
    });

    hide_cmd();
    run_message_loop();

    Ok(())
}

pub fn hide_cmd() {
    let window = unsafe { GetConsoleWindow() };

    if !window.is_null() {
        unsafe { ShowWindow(window, SW_HIDE) };
    }
}

fn build_menu(e: Event) -> MenuBuilder<Event> {
    MenuBuilder::new()
        .checkable("sRGB (Disable)", e == Event::SetSRGB, Event::SetSRGB)
        .separator()
        .checkable("Gamma 2.0", e == Event::SetG20, Event::SetG20)
        .checkable("Gamma 2.2", e == Event::SetG22, Event::SetG22)
        .checkable("Gamma 2.4", e == Event::SetG24, Event::SetG24)
        .separator()
        .item("Exit", Event::Exit)
}

fn run_message_loop() {
    loop {
        let mut msg = MaybeUninit::uninit();

        unsafe {
            if winuser::GetMessageA(msg.as_mut_ptr(), 0 as _, 0, 0) > 0 {
                winuser::TranslateMessage(msg.as_ptr());
                winuser::DispatchMessageA(msg.as_ptr());
            } else {
                break;
            }
        }
    }
}
