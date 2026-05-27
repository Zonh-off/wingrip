use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU32, Ordering};
use crate::config::types::Config;

// Global lock-free configuration atomics for high-performance zero-copy lookups
pub static ATOMIC_DEADZONE_PIXELS: AtomicI32 = AtomicI32::new(8);
pub static ATOMIC_SNAPPING_THRESHOLD_PIXELS: AtomicI32 = AtomicI32::new(10);
pub static ATOMIC_GAP_PIXELS: AtomicI32 = AtomicI32::new(8);
pub static ATOMIC_PREVIEW_FILL_COLOR: AtomicU32 = AtomicU32::new(0x00B98029);
pub static ATOMIC_PREVIEW_BORDER_COLOR: AtomicU32 = AtomicU32::new(0x00DB9834);
pub static ATOMIC_PREVIEW_OPACITY: AtomicU8 = AtomicU8::new(120);
pub static ATOMIC_PREVIEW_BORDER_RADIUS: AtomicI32 = AtomicI32::new(8);
pub static ATOMIC_LAYOUTS_ENABLED: AtomicBool = AtomicBool::new(true);
pub static ATOMIC_GESTURES_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn sync_atomics(config: &Config) {
    ATOMIC_DEADZONE_PIXELS.store(config.settings.deadzone_pixels, Ordering::Relaxed);
    ATOMIC_SNAPPING_THRESHOLD_PIXELS
        .store(config.settings.snapping_threshold_pixels, Ordering::Relaxed);
    ATOMIC_LAYOUTS_ENABLED.store(config.settings.layouts_enabled.unwrap_or(true), Ordering::Relaxed);
    ATOMIC_GESTURES_ENABLED.store(config.settings.gestures_enabled.unwrap_or(true), Ordering::Relaxed);

    let ui = config.get_ui_config();
    ATOMIC_GAP_PIXELS.store(ui.gap_pixels, Ordering::Relaxed);
    ATOMIC_PREVIEW_FILL_COLOR.store(ui.preview_fill_color, Ordering::Relaxed);
    ATOMIC_PREVIEW_BORDER_COLOR.store(ui.preview_border_color, Ordering::Relaxed);
    ATOMIC_PREVIEW_OPACITY.store(ui.preview_opacity, Ordering::Relaxed);
    ATOMIC_PREVIEW_BORDER_RADIUS.store(ui.preview_border_radius, Ordering::Relaxed);
}
