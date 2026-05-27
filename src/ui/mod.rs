pub mod types;
pub mod wnd_proc;

pub use types::*;
pub use wnd_proc::*;

use crossbeam_channel::Receiver;
use std::sync::atomic::Ordering;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, WPARAM, HINSTANCE};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DispatchMessageW, GetMessageW, LoadCursorW, PostMessageW, RegisterClassW,
    SetLayeredWindowAttributes, TranslateMessage, IDC_ARROW, LWA_ALPHA, MSG,
    WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

/// Initializes overlay window class registration, layered attributes setup, and manages
/// the standard Win32 Message Loop to paint and move visual previews dynamically.
pub fn run_ui_loop(rx: Receiver<UiEvent>) -> Result<(), Box<dyn std::error::Error>> {
    let class_name = windows::core::w!("WingripOverlayClass");

    unsafe {
        let module_handle = GetModuleHandleW(None)?;

        // Register window class for overlay
        let wnd_class = WNDCLASSW {
            style: windows::Win32::UI::WindowsAndMessaging::CS_HREDRAW
                | windows::Win32::UI::WindowsAndMessaging::CS_VREDRAW,
            lpfnWndProc: Some(overlay_wnd_proc),
            hInstance: HINSTANCE(module_handle.0),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };

        RegisterClassW(&wnd_class);

        // Create layered, topmost click-through borderless window
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            PCWSTR(class_name.as_ptr()),
            windows::core::w!("Wingrip Zone Preview"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            HWND::default(),
            None,
            module_handle,
            None,
        )?;

        if hwnd.0.is_null() {
            return Err("Failed to create visual overlay window".into());
        }

        let preview_opacity = crate::config::ATOMIC_PREVIEW_OPACITY.load(Ordering::Relaxed);
        // Configure alpha blending transparency dynamically
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), preview_opacity, LWA_ALPHA);

        // Store the HWND reference globally so the logic thread can post messages to it directly
        if let Ok(mut guard) = OVERLAY_HWND.lock() {
            *guard = SafeHwnd(hwnd);
        }

        println!("[OK] Visual Overlay GDI Window initialized successfully.");

        // Spawn a background helper thread to consume the traditional UiEvent channel structure
        // and translate it to native high-performance Window Messages (PostMessageW).
        // This preserves the main.rs thread layout while keeping CPU idle time at exactly 0.0%.
        let hwnd_raw = hwnd.0 as usize;
        std::thread::spawn(move || {
            let overlay_hwnd = HWND(hwnd_raw as *mut std::ffi::c_void);
            while let Ok(event) = rx.recv() {
                match event {
                    UiEvent::ShowPreview { rect } => {
                        if let Ok(mut rect_guard) = SNAPPING_RECT.lock() {
                            *rect_guard = Some(rect);
                        }
                        let _ = PostMessageW(overlay_hwnd, WM_USER_SNAP_PREVIEW, WPARAM(1), LPARAM(0));
                    }
                    UiEvent::HidePreview => {
                        let _ = PostMessageW(overlay_hwnd, WM_USER_SNAP_PREVIEW, WPARAM(0), LPARAM(0));
                    }
                }
            }
        });

        // Run standard Win32 Message Pump loop to feed GDI paint routines
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        }
    }

    Ok(())
}
