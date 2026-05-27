pub mod types;
pub mod callbacks;
pub mod watchdog;

pub use types::*;
pub use callbacks::*;
pub use watchdog::*;

use crossbeam_channel::Sender;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, TranslateMessage, DispatchMessageW,
    HHOOK, MSG, WH_MOUSE_LL, WH_KEYBOARD_LL,
};

const WM_USER: u32 = 0x0400;
const WM_REHOOK: u32 = WM_USER + 100;

/// Hooks global OS mouse coordinates, establishes standard Win32 Message pump loop to route callbacks,
/// and unregisters resources gracefully when process signals exit.
pub fn run_input_hook(tx: Sender<InputEvent>) -> Result<(), Box<dyn std::error::Error>> {
    // Store the global sender
    if EVENT_SENDER.set(tx).is_err() {
        return Err("Failed to register EVENT_SENDER globally".into());
    }

    unsafe {
        let module_handle = GetModuleHandleW(None)?;
        
        // Register low-level mouse hook globally
        let mut hook_handle: HHOOK = SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(low_level_mouse_proc),
            module_handle,
            0,
        )?;

        if hook_handle.0.is_null() {
            return Err("Failed to register WH_MOUSE_LL hook handle".into());
        }

        // Register low-level keyboard hook globally
        let mut kbd_hook_handle: HHOOK = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(low_level_keyboard_proc),
            module_handle,
            0,
        )?;

        if kbd_hook_handle.0.is_null() {
            let _ = UnhookWindowsHookEx(hook_handle);
            return Err("Failed to register WH_KEYBOARD_LL hook handle".into());
        }

        println!("[OK] Low-Level Mouse and Keyboard Hooks registered successfully.");

        // Capture main thread ID to post recovery messages from watchdog thread
        let main_thread_id = windows::Win32::System::Threading::GetCurrentThreadId();

        // Start background watchdog thread to auto-recover hooks after lock screen / sleep
        std::thread::spawn(move || {
            run_watchdog(main_thread_id, WM_REHOOK);
        });

        // Start Win32 message loop to feed global callback queue (required for LL hooks)
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            if msg.message == WM_REHOOK {
                println!("[WATCHDOG] Rehooking low-level input hooks dynamically...");
                // Unhook old handles
                if !hook_handle.0.is_null() {
                    let _ = UnhookWindowsHookEx(hook_handle);
                }
                if !kbd_hook_handle.0.is_null() {
                    let _ = UnhookWindowsHookEx(kbd_hook_handle);
                }

                // Register mouse hook
                if let Ok(new_hook) = SetWindowsHookExW(
                    WH_MOUSE_LL,
                    Some(low_level_mouse_proc),
                    module_handle,
                    0,
                ) {
                    hook_handle = new_hook;
                } else {
                    eprintln!("[ERROR] Failed to recover mouse hook.");
                }

                // Register keyboard hook
                if let Ok(new_kbd_hook) = SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(low_level_keyboard_proc),
                    module_handle,
                    0,
                ) {
                    kbd_hook_handle = new_kbd_hook;
                } else {
                    eprintln!("[ERROR] Failed to recover keyboard hook.");
                }
                println!("[OK] Input hooks recovered and re-registered successfully.");
            } else {
                let _ = TranslateMessage(&msg);
                let _ = DispatchMessageW(&msg);
            }
        }

        // Cleanup hooks on loop exit
        let _ = UnhookWindowsHookEx(hook_handle);
        let _ = UnhookWindowsHookEx(kbd_hook_handle);
    }

    Ok(())
}
