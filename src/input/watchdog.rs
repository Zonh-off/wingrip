use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW;

pub fn run_watchdog(main_thread_id: u32, wm_rehook: u32) {
    let mut was_locked = false;
    let mut last_tick = std::time::Instant::now();
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3));

        let now = std::time::Instant::now();
        let elapsed_secs = now.duration_since(last_tick).as_secs();
        last_tick = now;

        // 1. Detect sleep/resume if sleep time exceeded 10 seconds
        let sleep_resume_detected = elapsed_secs > 10;

        // 2. Detect lock state using access privileges to secure desktop (fails when locked)
        let is_unlocked = {
            let result = unsafe {
                windows::Win32::System::StationsAndDesktops::OpenInputDesktop(
                    windows::Win32::System::StationsAndDesktops::DESKTOP_CONTROL_FLAGS(0),
                    false,
                    windows::Win32::System::StationsAndDesktops::DESKTOP_SWITCHDESKTOP,
                )
            };
            match result {
                Ok(hdesk) => {
                    let _ = unsafe { windows::Win32::System::StationsAndDesktops::CloseDesktop(hdesk) };
                    true
                }
                Err(_) => false,
            }
        };

        let should_rehook = if sleep_resume_detected {
            println!("[WATCHDOG] System sleep/resume detected. Re-registering hooks...");
            true
        } else if !is_unlocked {
            was_locked = true;
            false
        } else if was_locked {
            println!("[WATCHDOG] System unlock detected. Re-registering hooks...");
            was_locked = false;
            true
        } else {
            false
        };

        if should_rehook {
            unsafe {
                let _ = PostThreadMessageW(
                    main_thread_id,
                    wm_rehook,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        }
    }
}
