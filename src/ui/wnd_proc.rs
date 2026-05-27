use std::sync::atomic::Ordering;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateRoundRectRgn, CreateSolidBrush, DeleteObject, EndPaint, FillRgn, FrameRgn,
    PAINTSTRUCT, HRGN, SetWindowRgn,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, GetClientRect, SetLayeredWindowAttributes, SetWindowPos, ShowWindow,
    HWND_TOPMOST, LWA_ALPHA, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE,
};
use crate::ui::types::{WM_USER_SNAP_PREVIEW, SNAPPING_RECT};

/// Custom Window Procedure WndProc for the semi-transparent GDI overlay preview window.
/// Handles drawing, flicker mitigation, and coordinates sizing notifications dynamically.
pub unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_USER_SNAP_PREVIEW => {
                if wparam.0 == 1 {
                    // Show/Update position of the visual preview container
                    if let Ok(guard) = SNAPPING_RECT.lock() {
                        if let Some(rect) = *guard {
                            let x = rect.left;
                            let y = rect.top;
                            let w = rect.right - rect.left;
                            let h = rect.bottom - rect.top;

                            let preview_opacity = crate::config::ATOMIC_PREVIEW_OPACITY.load(Ordering::Relaxed);
                            let preview_border_radius = crate::config::ATOMIC_PREVIEW_BORDER_RADIUS.load(Ordering::Relaxed);

                            // Update opacity layer attribute dynamically to support instant config hot-reloading
                            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), preview_opacity, LWA_ALPHA);

                            let _ = SetWindowPos(
                                hwnd,
                                HWND_TOPMOST,
                                x,
                                y,
                                w,
                                h,
                                SWP_NOACTIVATE | SWP_SHOWWINDOW,
                            );

                            // Apply rounded corners dynamically to overlay shape
                            if preview_border_radius > 0 {
                                let hrgn = CreateRoundRectRgn(
                                    0,
                                    0,
                                    w,
                                    h,
                                    preview_border_radius,
                                    preview_border_radius,
                                );
                                let _ = SetWindowRgn(hwnd, hrgn, true);
                            } else {
                                let _ = SetWindowRgn(hwnd, HRGN::default(), true);
                            }
                        }
                    }
                } else {
                    // Hide the visual preview container
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
                LRESULT(0)
            }
            windows::Win32::UI::WindowsAndMessaging::WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);

                let mut rect = RECT::default();
                let _ = GetClientRect(hwnd, &mut rect);

                let preview_border_radius = crate::config::ATOMIC_PREVIEW_BORDER_RADIUS.load(Ordering::Relaxed);
                let preview_fill_color = crate::config::ATOMIC_PREVIEW_FILL_COLOR.load(Ordering::Relaxed);
                let preview_border_color = crate::config::ATOMIC_PREVIEW_BORDER_COLOR.load(Ordering::Relaxed);

                // Create a rounding region matching client coordinates
                let hrgn = CreateRoundRectRgn(
                    0,
                    0,
                    rect.right,
                    rect.bottom,
                    preview_border_radius,
                    preview_border_radius,
                );

                // 1. Draw smooth rounded interior solid fill (opacity is configured globally)
                let fill_brush = CreateSolidBrush(COLORREF(preview_fill_color));
                let _ = FillRgn(hdc, hrgn, fill_brush);
                let _ = DeleteObject(fill_brush);

                // 2. Draw thick sleek rounded outline highlight border
                let border_brush = CreateSolidBrush(COLORREF(preview_border_color));
                let _ = FrameRgn(hdc, hrgn, border_brush, 2, 2); // 2px thick border
                let _ = DeleteObject(border_brush);

                // Delete our temporary regional paint handle to prevent leaks
                let _ = DeleteObject(hrgn);

                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            windows::Win32::UI::WindowsAndMessaging::WM_ERASEBKGND => {
                LRESULT(1) // Avoid GDI rendering flickers
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
