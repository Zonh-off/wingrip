use windows::Win32::Foundation::{HWND, RECT, POINT};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::UI::WindowsAndMessaging::{
    GA_ROOT, GetAncestor, GetClassNameW, GetWindowRect, WindowFromPoint, IsWindowVisible, IsIconic, IsHungAppWindow,
};

/// Retrieves the invisible window shadow margins of a window on Windows 10/11 using DWM,
/// allowing us to perfectly expand snapped window coordinates to align edge-to-edge.
pub fn get_window_shadow_margins(hwnd: HWND) -> RECT {
    let mut window_rect = RECT::default();
    let mut frame_rect = RECT::default();

    if unsafe { GetWindowRect(hwnd, &mut window_rect) }.is_ok() {
        let hr = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut frame_rect as *mut RECT as *mut _,
                std::mem::size_of::<RECT>() as u32,
            )
        };
        if hr.is_ok() {
            return RECT {
                left: frame_rect.left - window_rect.left,
                top: frame_rect.top - window_rect.top,
                right: window_rect.right - frame_rect.right,
                bottom: window_rect.bottom - frame_rect.bottom,
            };
        }
    }

    // Default fallback margins if DWM call fails
    RECT {
        left: 7,
        top: 0,
        right: 7,
        bottom: 7,
    }
}

/// Comprehensive safety validation to verify if a window is a valid target for dragging/resizing
pub fn is_valid_target_window(hwnd: HWND) -> bool {
    if hwnd.0.is_null() {
        return false;
    }
    unsafe {
        // 1. Must be a visible window
        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }

        // 2. Must not be minimized (iconic)
        if IsIconic(hwnd).as_bool() {
            return false;
        }

        // 3. Must not be unresponsive / hung
        if IsHungAppWindow(hwnd).as_bool() {
            return false;
        }

        // 4. Must not be cloaked by DWM (invisible/ghost window)
        let mut cloaked: u32 = 0;
        let hr = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut _,
            std::mem::size_of::<u32>() as u32,
        );
        if hr.is_ok() && cloaked != 0 {
            return false;
        }

        // 5. Must not be a shell desktop layer, taskbar, or wingrip UI window
        let mut class_name = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut class_name);
        if len > 0 {
            let name = String::from_utf16_lossy(&class_name[..len as usize]);
            let name_lower = name.to_lowercase();
            if name_lower == "progman"
                || name_lower == "workerw"
                || name_lower == "shell_traywnd"
                || name_lower == "shell_secondarytraywnd"
                || name_lower == "wingripsettingsfluent"
                || name_lower == "wingripoverlay"
            {
                return false;
            }
        }
    }
    true
}

/// Helper function to retrieve topmost application window handle (HWND) at the specified screen coordinates
pub fn get_top_level_window_at(x: i32, y: i32) -> HWND {
    unsafe {
        let hwnd = WindowFromPoint(POINT { x, y });
        if hwnd.0.is_null() {
            return hwnd;
        }
        let root = GetAncestor(hwnd, GA_ROOT);
        if !is_valid_target_window(root) {
            return HWND::default();
        }
        root
    }
}
