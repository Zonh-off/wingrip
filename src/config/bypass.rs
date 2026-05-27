use std::path::Path;
use windows::Win32::Foundation::{CloseHandle, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Shell::{
    QUNS_APP, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
    SHQueryUserNotificationState,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetWindowRect, GetWindowThreadProcessId,
};
use windows::core::PWSTR;
use crate::config::types::Config;

impl Config {
    /// Determines whether the low-level hook should bypass intercepting inputs for a given window,
    /// by combining dynamic fullscreen checks, D3D fullscreen shell queries, and the TOML blacklist.
    pub fn should_bypass_window(&self, hwnd: HWND) -> bool {
        if hwnd.0.is_null() {
            return false;
        }

        // 1. Dynamic check: is the foreground window class Desktop or Taskbar?
        let mut class_name = [0u16; 256];
        let len = unsafe { GetClassNameW(hwnd, &mut class_name) };
        if len > 0 {
            let class_str = String::from_utf16_lossy(&class_name[..len as usize]);
            let class_lower = class_str.to_lowercase();
            // Ignore desktop shell layers so we don't accidentally mark the empty desktop as fullscreen game
            if class_lower == "progman"
                || class_lower == "workerw"
                || class_lower == "shell_traywnd"
            {
                return false;
            }
        }

        // 2. Query Windows Shell notification state (D3D fullscreen game/app checking)
        unsafe {
            if let Ok(state) = SHQueryUserNotificationState() {
                match state {
                    QUNS_RUNNING_D3D_FULL_SCREEN
                    | QUNS_BUSY
                    | QUNS_PRESENTATION_MODE
                    | QUNS_APP => {
                        return true;
                    }
                    _ => {}
                }
            }
        }

        // 3. Dynamic layout check: Does the active window bounds match or exceed the monitor size?
        unsafe {
            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).is_ok() {
                let hmonitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                if !hmonitor.0.is_null() {
                    let mut monitor_info = MONITORINFO {
                        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                        ..Default::default()
                    };
                    if GetMonitorInfoW(hmonitor, &mut monitor_info).as_bool() {
                        let mon_rect = monitor_info.rcMonitor;
                        let is_fs = rect.left <= mon_rect.left
                            && rect.top <= mon_rect.top
                            && rect.right >= mon_rect.right
                            && rect.bottom >= mon_rect.bottom;
                        if is_fs {
                            return true;
                        }
                    }
                }
            }
        }

        // 4. Static process check: Retrieve process executable name and check the manual blacklist
        unsafe {
            let mut process_id = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut process_id));
            if process_id != 0 {
                if let Ok(hprocess) =
                    OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id)
                {
                    let mut size = 260u32;
                    let mut buffer = vec![0u16; size as usize];
                    if QueryFullProcessImageNameW(
                        hprocess,
                        PROCESS_NAME_FORMAT(0),
                        PWSTR(buffer.as_mut_ptr()),
                        &mut size,
                    )
                    .is_ok()
                    {
                        let path = String::from_utf16_lossy(&buffer[..size as usize]);
                        if let Some(exe_name) =
                            Path::new(&path).file_name().and_then(|n| n.to_str())
                        {
                            if self.is_blacklisted(exe_name) {
                                let _ = CloseHandle(hprocess);
                                return true;
                            }
                        }
                    }
                    let _ = CloseHandle(hprocess);
                }
            }
        }

        false
    }
}
