use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use windows::Win32::Foundation::{HWND, RECT};

// Global thread-safe cache to remember the pre-snapped floating size of windows.
pub static PRE_SNAP_RECTS: Lazy<Mutex<HashMap<isize, RECT>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// Global thread-safe registry of currently active snapped windows and their targeted logic zones.
pub static SNAPPED_WINDOWS: Lazy<Mutex<HashMap<isize, RECT>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveRegion {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationMode {
    Idle,
    Dragging {
        hwnd: HWND,
        start_cursor: (i32, i32),
        start_window_rect: RECT,
        has_passed_deadzone: bool,
        is_zoning: bool,
    },
    Resizing {
        hwnd: HWND,
        start_cursor: (i32, i32),
        start_window_rect: RECT,
        active_region: ActiveRegion,
        has_passed_deadzone: bool,
    },
}
