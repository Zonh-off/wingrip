use crate::config::CONFIG;
use crate::input::types::{InputEvent, MouseAction, MouseButton};
use crossbeam_channel::Sender;
use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_LWIN, VK_RWIN, VK_SHIFT, keybd_event, KEYEVENTF_KEYUP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetForegroundWindow, MSLLHOOKSTRUCT, KBDLLHOOKSTRUCT,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP,
    WM_KEYUP, WM_SYSKEYUP,
};

// Global thread-safe channel sender for hook callback communication
pub static EVENT_SENDER: OnceCell<Sender<InputEvent>> = OnceCell::new();

// Shared atomic flag allowing the logic thread to signal that window manipulation is currently active,
// ensuring the hook continues to intercept mouse button release events even if the modifier key is released early.
pub static IS_OPERATION_ACTIVE: AtomicBool = AtomicBool::new(false);
pub static WAS_OPERATION_PERFORMED: AtomicBool = AtomicBool::new(false);

/// Dynamic low-level Windows mouse hook callback function.
/// Processes coordinates in constant time O(1) and dispatches clean event structures to the Logic channel thread.
pub unsafe extern "system" fn low_level_mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        unsafe {
            // Fast lock-free pre-filter: check if the modifier key or an active drag/resize operation is running.
            // If neither is true, immediately pass control downstream without calling APIs or locking mutexes.
            let win_pressed = GetAsyncKeyState(VK_LWIN.0 as i32) < 0 || GetAsyncKeyState(VK_RWIN.0 as i32) < 0;
            let is_active = win_pressed || IS_OPERATION_ACTIVE.load(Ordering::Relaxed);

            if !is_active {
                return CallNextHookEx(None, code, wparam, lparam);
            }

            // A. Strategy 1 Auto-Bypass: If a fullscreen game or Direct3D app is active, completely skip hook logic
            let active_hwnd = GetForegroundWindow();
            if !active_hwnd.0.is_null() && CONFIG.lock().unwrap().should_bypass_window(active_hwnd) {
                return CallNextHookEx(None, code, wparam, lparam);
            }

            // B. Check key states for global modifiers
            let shift_pressed = GetAsyncKeyState(VK_SHIFT.0 as i32) < 0;

            let info = *(lparam.0 as *const MSLLHOOKSTRUCT);
            let x = info.pt.x;
            let y = info.pt.y;

            let msg = wparam.0 as u32;

            // C. Start Menu Suppression: If the Win key is held down and the user presses or releases mouse buttons,
            // programmatically inject the official VK_NONAME (0xFC) dummy virtual keycode.
            // This marks the keyboard state as "shortcut executed", canceling the default Start Menu popup on Win release.
            if win_pressed && (
                msg == WM_LBUTTONDOWN || msg == WM_RBUTTONDOWN ||
                msg == WM_LBUTTONUP || msg == WM_RBUTTONUP
            ) {
                keybd_event(0xFC, 0, Default::default(), 0);
                keybd_event(0xFC, 0, KEYEVENTF_KEYUP, 0);
                WAS_OPERATION_PERFORMED.store(true, Ordering::Relaxed);
            }

            let mut event = None;

            match msg {
                WM_MOUSEMOVE => {
                    event = Some(InputEvent::MouseMove { x, y, shift_pressed });
                }
                WM_LBUTTONDOWN => {
                    event = Some(InputEvent::MouseButton {
                        button: MouseButton::Left,
                        action: MouseAction::Press,
                        x,
                        y,
                    });
                }
                WM_LBUTTONUP => {
                    event = Some(InputEvent::MouseButton {
                        button: MouseButton::Left,
                        action: MouseAction::Release,
                        x,
                        y,
                    });
                }
                WM_RBUTTONDOWN => {
                    event = Some(InputEvent::MouseButton {
                        button: MouseButton::Right,
                        action: MouseAction::Press,
                        x,
                        y,
                    });
                }
                WM_RBUTTONUP => {
                    event = Some(InputEvent::MouseButton {
                        button: MouseButton::Right,
                        action: MouseAction::Release,
                        x,
                        y,
                    });
                }
                _ => {}
            }

            if let Some(ev) = event {
                // Forward details instantly via lock-free queue to free up Hook thread execution
                if let Some(sender) = EVENT_SENDER.get() {
                    let _ = sender.send(ev);
                }

                // If window manipulation is active or modifier triggers, consume the button clicks
                // so they are not routed down to click links or highlight elements inside targeted windows.
                if win_pressed || IS_OPERATION_ACTIVE.load(Ordering::Relaxed) {
                    if msg == WM_LBUTTONDOWN
                        || msg == WM_LBUTTONUP
                        || msg == WM_RBUTTONDOWN
                        || msg == WM_RBUTTONUP
                    {
                        return LRESULT(1); // Consumed
                    }
                }
            }
        }
    }

    // Pass execution downstream
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// Low-level global keyboard hook callback to suppress the Start Menu when releasing the Win key.
pub unsafe extern "system" fn low_level_keyboard_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        unsafe {
            let msg = wparam.0 as u32;
            if msg == WM_KEYUP || msg == WM_SYSKEYUP {
                let info = *(lparam.0 as *const KBDLLHOOKSTRUCT);
                let vk_code = info.vkCode;

                if vk_code == VK_LWIN.0 as u32 || vk_code == VK_RWIN.0 as u32 {
                    if WAS_OPERATION_PERFORMED.load(Ordering::Relaxed) {
                        // Inject the dummy key precisely before the Win key is released to the OS
                        keybd_event(0xFC, 0, Default::default(), 0);
                        keybd_event(0xFC, 0, KEYEVENTF_KEYUP, 0);

                        WAS_OPERATION_PERFORMED.store(false, Ordering::Relaxed);
                    }
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}
