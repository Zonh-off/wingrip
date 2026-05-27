use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOW, SW_HIDE};
use crate::settings::state::{
    HWND_DEADZONE, HWND_THRESHOLD, HWND_GAP, HWND_RADIUS, HWND_OPACITY, HWND_BLACKLIST_EDIT,
};

pub fn update_controls_visibility(active_tab: usize) {
    unsafe {
        let show_tab0 = if active_tab == 0 { SW_SHOW } else { SW_HIDE };
        let show_tab1 = if active_tab == 1 { SW_SHOW } else { SW_HIDE };
        let show_tab2 = if active_tab == 2 { SW_SHOW } else { SW_HIDE };

        if let Some(h) = *HWND_DEADZONE.lock().unwrap() {
            let _ = ShowWindow(h.0, show_tab0);
        }
        if let Some(h) = *HWND_THRESHOLD.lock().unwrap() {
            let _ = ShowWindow(h.0, show_tab0);
        }
        if let Some(h) = *HWND_GAP.lock().unwrap() {
            let _ = ShowWindow(h.0, show_tab1);
        }
        if let Some(h) = *HWND_RADIUS.lock().unwrap() {
            let _ = ShowWindow(h.0, show_tab1);
        }
        if let Some(h) = *HWND_OPACITY.lock().unwrap() {
            let _ = ShowWindow(h.0, show_tab1);
        }
        if let Some(h) = *HWND_BLACKLIST_EDIT.lock().unwrap() {
            let _ = ShowWindow(h.0, show_tab2);
        }
    }
}
