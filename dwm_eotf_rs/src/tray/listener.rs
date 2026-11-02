use std::{
    mem::MaybeUninit,
    sync::{OnceLock, mpsc::Sender},
};

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::WindowsAndMessaging::{
            CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DispatchMessageA, GetMessageA,
            RegisterClassW, TranslateMessage, WM_DISPLAYCHANGE, WNDCLASS_STYLES, WNDCLASSW,
            WS_EX_LTRREADING, WS_OVERLAPPEDWINDOW,
        },
    },
    core::PCWSTR,
};

use crate::tray::Event;

static TX: OnceLock<Sender<Event>> = OnceLock::new();

#[allow(unused)]
unsafe extern "system" fn winproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_DISPLAYCHANGE {
        if let Some(tx) = TX.get() {
            let _ = tx.send(Event::DisplayChange);
        }
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

pub fn run_watcher_window(tx: Sender<Event>) {
    TX.set(tx).unwrap();

    let wnd_class_name: Vec<u16> = "dwm_eotf_listener".encode_utf16().collect();
    let wnd_name: Vec<u16> = "dwm_eotf_listener_window".encode_utf16().collect();

    let wnd_class = WNDCLASSW {
        style: WNDCLASS_STYLES(0),
        lpfnWndProc: Some(winproc),
        lpszClassName: PCWSTR::from_raw(wnd_class_name.as_ptr()),
        ..Default::default()
    };

    unsafe { RegisterClassW(&wnd_class) };

    let _ = unsafe {
        CreateWindowExW(
            WS_EX_LTRREADING,
            PCWSTR::from_raw(wnd_class_name.as_ptr()),
            PCWSTR::from_raw(wnd_name.as_ptr()),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            None,
            None,
        )
        .unwrap()
    };

    let mut lpmsg = MaybeUninit::uninit();

    unsafe {
        while GetMessageA(lpmsg.as_mut_ptr(), None, 0, 0).0 > 0 {
            let _ = TranslateMessage(lpmsg.as_ptr());
            DispatchMessageA(lpmsg.as_ptr());
        }
    }
}
