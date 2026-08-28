
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, MOD_CONTROL, MOD_SHIFT, VK_CONTROL, VK_E, VK_V,
};
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetCursorPos, GetForegroundWindow, GetMessageW, SetForegroundWindow, MSG,
    WM_HOTKEY,
};

const ID_RACCOURCI: i32 = 1;

pub fn preparer_session() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

pub fn armer_raccourci(toggle_requested: Arc<AtomicBool>, ctx: eframe::egui::Context) {
    std::thread::spawn(move || unsafe {
        if RegisterHotKey(None, ID_RACCOURCI, MOD_CONTROL | MOD_SHIFT, VK_E.0 as u32).is_err() {
            eprintln!("Avertissement: échec de l'enregistrement du raccourci Ctrl+Shift+E");
            return;
        }

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            if message.message == WM_HOTKEY && message.wParam.0 as i32 == ID_RACCOURCI {
                toggle_requested.store(true, Ordering::SeqCst);
                ctx.request_repaint();
            }
        }
    });
}

pub fn position_souris() -> Option<(i32, i32)> {
    let mut point = windows::Win32::Foundation::POINT::default();
    unsafe { GetCursorPos(&mut point).ok()? };
    Some((point.x, point.y))
}

pub fn fenetre_active() -> Option<String> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        None
    } else {
        Some((hwnd.0 as isize).to_string())
    }
}

pub fn forcer_focus_fenetre_app() {
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(80));
        unsafe {
            let titre: Vec<u16> = "Ruji\0".encode_utf16().collect();
            let hwnd = FindWindowW(PCWSTR::null(), PCWSTR(titre.as_ptr()));
            if let Ok(hwnd) = hwnd {
                let _ = SetForegroundWindow(hwnd);
            }
        }
    });
}

pub fn coller_texte(texte: &str, fenetre_precedente: Option<String>) {
    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        eprintln!("Avertissement: impossible d'accéder au presse-papier");
        return;
    };
    if let Err(e) = clipboard.set_text(texte) {
        eprintln!("Avertissement: échec de la copie dans le presse-papier: {e}");
        return;
    }

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(120));
        if let Some(id) = fenetre_precedente.and_then(|s| s.parse::<isize>().ok()) {
            unsafe {
                let _ = SetForegroundWindow(HWND(id as *mut core::ffi::c_void));
            }
            std::thread::sleep(std::time::Duration::from_millis(60));
        }
        envoyer_ctrl_v();
    });
}

fn envoyer_ctrl_v() {
    let touche = |code: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY,
                   relache: bool|
     -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: code,
                    wScan: 0,
                    dwFlags: if relache { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    };

    let evenements = [
        touche(VK_CONTROL, false),
        touche(VK_V, false),
        touche(VK_V, true),
        touche(VK_CONTROL, true),
    ];

    unsafe {
        SendInput(&evenements, std::mem::size_of::<INPUT>() as i32);
    }
}
