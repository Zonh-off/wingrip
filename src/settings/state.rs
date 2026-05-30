use crate::config::CONFIG;
pub use crate::ui::SafeHwnd;
use once_cell::sync::Lazy;
use std::sync::Mutex;

pub use crate::settings::types::*;
pub use crate::settings::actions::*;

pub static STATE: Lazy<Mutex<DashboardState>> = Lazy::new(|| {
    let cfg = CONFIG.lock().unwrap();
    let ui_cfg = cfg.get_ui_config();
    Mutex::new(DashboardState {
        active_tab: 0,
        hovered: HoveredControl::None,
        deadzone: cfg.settings.deadzone_pixels,
        threshold: cfg.settings.snapping_threshold_pixels,
        gap: ui_cfg.gap_pixels,
        radius: ui_cfg.preview_border_radius,
        opacity: ui_cfg.preview_opacity,
        fill_color: ui_cfg.preview_fill_color,
        border_color: ui_cfg.preview_border_color,
        blacklist_text: cfg.blacklist.processes.join("\r\n"),
        startup_enabled: crate::config::registry::is_startup_enabled(),
        layouts_enabled: cfg.settings.layouts_enabled.unwrap_or(true),
        gestures_enabled: cfg.settings.gestures_enabled.unwrap_or(true),
        split_zones_enabled: cfg.settings.split_zones_enabled.unwrap_or(true),
    })
});

pub static SETTINGS_HWND: Lazy<Mutex<Option<SafeHwnd>>> = Lazy::new(|| Mutex::new(None));
pub static HWND_BLACKLIST_EDIT: Lazy<Mutex<Option<SafeHwnd>>> = Lazy::new(|| Mutex::new(None));
pub static HWND_DEADZONE: Lazy<Mutex<Option<SafeHwnd>>> = Lazy::new(|| Mutex::new(None));
pub static HWND_THRESHOLD: Lazy<Mutex<Option<SafeHwnd>>> = Lazy::new(|| Mutex::new(None));
pub static HWND_GAP: Lazy<Mutex<Option<SafeHwnd>>> = Lazy::new(|| Mutex::new(None));
pub static HWND_RADIUS: Lazy<Mutex<Option<SafeHwnd>>> = Lazy::new(|| Mutex::new(None));
pub static HWND_OPACITY: Lazy<Mutex<Option<SafeHwnd>>> = Lazy::new(|| Mutex::new(None));
pub static INPUT_BG_BRUSH: Lazy<Mutex<Option<SafeHbrush>>> = Lazy::new(|| Mutex::new(None));
