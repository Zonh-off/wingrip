use std::sync::Mutex;
use windows::Win32::Foundation::{HWND, RECT};

// Custom Win32 user message to notify UI overlay of snapping boundaries changes
pub const WM_USER_SNAP_PREVIEW: u32 = 0x0400 + 1; // WM_USER + 1

// Thread-safe global references for high-performance zero-copy preview boundaries
pub static SNAPPING_RECT: Mutex<Option<RECT>> = Mutex::new(None);

#[derive(Debug, Clone, Copy)]
pub struct SafeHwnd(pub HWND);

// Manually implement Send and Sync to allow storing in static thread-safe contexts safely
unsafe impl Send for SafeHwnd {}
unsafe impl Sync for SafeHwnd {}

pub static OVERLAY_HWND: Mutex<SafeHwnd> = Mutex::new(SafeHwnd(HWND(std::ptr::null_mut())));

#[derive(Debug, Clone, Copy)]
pub enum UiEvent {
    ShowPreview { rect: RECT },
    HidePreview,
}
