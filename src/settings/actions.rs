use std::fs;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowTextLengthW, GetWindowTextW, MessageBoxW, MB_OK, MB_ICONINFORMATION,
};
use windows::core::w;

use crate::config::{Config, Settings, Blacklist, UiConfig};
use crate::settings::state::{
    STATE, HWND_DEADZONE, HWND_THRESHOLD, HWND_GAP, HWND_RADIUS, HWND_OPACITY, HWND_BLACKLIST_EDIT,
};
use crate::ui::SafeHwnd;

pub fn commit_settings_action(hwnd: HWND) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = STATE.lock().unwrap();

    // Read and parse all custom numeric input box values dynamically
    unsafe {
        let read_edit_val = |opt_hwnd: Option<SafeHwnd>, default_val: i32| -> i32 {
            if let Some(h) = opt_hwnd {
                let len = GetWindowTextLengthW(h.0);
                if len > 0 {
                    let mut buf = vec![0u16; (len + 1) as usize];
                    let read = GetWindowTextW(h.0, &mut buf);
                    if let Ok(text) = String::from_utf16(&buf[..read as usize]) {
                        if let Ok(val) = text.trim().parse::<i32>() {
                            return val;
                        }
                    }
                }
            }
            default_val
        };

        state.deadzone = read_edit_val(*HWND_DEADZONE.lock().unwrap(), state.deadzone);
        state.threshold = read_edit_val(*HWND_THRESHOLD.lock().unwrap(), state.threshold);
        state.gap = read_edit_val(*HWND_GAP.lock().unwrap(), state.gap);
        state.radius = read_edit_val(*HWND_RADIUS.lock().unwrap(), state.radius);

        let opacity_val = read_edit_val(*HWND_OPACITY.lock().unwrap(), state.opacity as i32);
        state.opacity = opacity_val.clamp(10, 255) as u8;
    }

    // 1. Read blacklist process text from the multi-line edit box if active
    if let Ok(guard) = HWND_BLACKLIST_EDIT.lock() {
        if let Some(ref safe_edit) = *guard {
            unsafe {
                let len = GetWindowTextLengthW(safe_edit.0);
                if len > 0 {
                    let mut buf = vec![0u16; (len + 1) as usize];
                    let read = GetWindowTextW(safe_edit.0, &mut buf);
                    state.blacklist_text = String::from_utf16_lossy(&buf[..read as usize]);
                } else {
                    state.blacklist_text = String::new();
                }
            }
        }
    }

    let processes: Vec<String> = state
        .blacklist_text
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    // 2. Re-create new config struct from live dashboard states
    let updated = Config {
        settings: Settings {
            deadzone_pixels: state.deadzone,
            snapping_threshold_pixels: state.threshold,
            layouts_enabled: Some(state.layouts_enabled),
            gestures_enabled: Some(state.gestures_enabled),
        },
        blacklist: Blacklist { processes },
        ui: Some(UiConfig {
            preview_fill_color: state.fill_color,
            preview_border_color: state.border_color,
            preview_opacity: state.opacity,
            preview_border_radius: state.radius,
            gap_pixels: state.gap,
        }),
    };

    // 3. Serialize to TOML
    let toml_string = toml::to_string_pretty(&updated)?;

    // 4. Resolve exact target path dynamically
    let mut resolved_path = None;
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let path = exe_dir.join("config.toml");
            if path.exists() {
                resolved_path = Some(path);
            } else {
                let mut current = exe_dir.to_path_buf();
                for _ in 0..3 {
                    if let Some(parent) = current.parent() {
                        let path = parent.join("config.toml");
                        if path.exists() {
                            resolved_path = Some(path);
                            break;
                        }
                        current = parent.to_path_buf();
                    } else {
                        break;
                    }
                }
            }
        }
    }

    let save_path = resolved_path.unwrap_or_else(|| std::path::PathBuf::from("config.toml"));

    // 5. Write to file and dynamic hot-reload instantly
    fs::write(&save_path, toml_string)?;
    crate::config::reload_config();

    // 6. Apply Windows startup registration dynamically
    if let Err(e) = crate::config::registry::set_startup_enabled(state.startup_enabled) {
        eprintln!("[ERROR] Failed to update startup registry key: {:?}", e);
    }

    // Symmetrical native Alert popup
    unsafe {
        MessageBoxW(
            hwnd,
            w!("Configurations saved successfully and hot-reloaded dynamically!"),
            w!("wingrip Settings"),
            MB_OK | MB_ICONINFORMATION,
        );
    }

    Ok(())
}
