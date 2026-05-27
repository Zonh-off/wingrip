use std::sync::atomic::Ordering;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    MONITOR_DEFAULTTONEAREST, MonitorFromWindow,
};
use crate::logic::SNAPPED_WINDOWS;

pub fn calculate_balanced_rect(rect: RECT, work: RECT, gap: i32) -> RECT {
    let mut balanced = rect;

    // Left boundary: Screen edge vs. inner division boundary
    if rect.left == work.left {
        balanced.left = rect.left + gap;
    } else {
        balanced.left = rect.left + gap / 2;
    }

    // Right boundary: Screen edge vs. inner division boundary
    if rect.right == work.right {
        balanced.right = rect.right - gap;
    } else {
        balanced.right = rect.right - gap / 2;
    }

    // Top boundary: Screen edge vs. inner division boundary
    if rect.top == work.top {
        balanced.top = rect.top + gap;
    } else {
        balanced.top = rect.top + gap / 2;
    }

    // Bottom boundary: Screen edge vs. inner division boundary
    if rect.bottom == work.bottom {
        balanced.bottom = rect.bottom - gap;
    } else {
        balanced.bottom = rect.bottom - gap / 2;
    }

    balanced
}

/// Dynamic side-by-side and top-to-bottom gap auto-filler snapping resolver
pub fn adjust_rect_for_adjacent_snapped_windows(mut rect: RECT, hwnd: HWND, work: RECT) -> RECT {
    if !crate::config::ATOMIC_LAYOUTS_ENABLED.load(Ordering::Relaxed) {
        return rect;
    }
    unsafe {
        let active_monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if active_monitor.0.is_null() {
            return rect;
        }

        if let Ok(guard) = SNAPPED_WINDOWS.lock() {
            for (&other_hwnd_val, &other_rect) in guard.iter() {
                // Skip ourselves
                if other_hwnd_val == hwnd.0 as isize {
                    continue;
                }

                let other_hwnd = HWND(other_hwnd_val as *mut _);
                let other_monitor = MonitorFromWindow(other_hwnd, MONITOR_DEFAULTTONEAREST);
                if other_monitor != active_monitor {
                    continue;
                }

                // Check side-by-side alignment (overlap in Y axis)
                let overlap_y = rect.bottom.min(other_rect.bottom) - rect.top.max(other_rect.top);
                if overlap_y > 100 {
                    // Case 1: Our target is left-aligned, and the other is right-aligned
                    // (Allow small tolerance for shadows/margins, e.g. 50 pixels)
                    if (rect.left - work.left).abs() <= 50 && (other_rect.right - work.right).abs() <= 50 {
                        // Snap our right edge exactly to the other's left boundary!
                        if other_rect.left > work.left + 100 && other_rect.left < work.right - 100 {
                            rect.right = other_rect.left;
                        }
                    }
                    // Case 2: Our target is right-aligned, and the other is left-aligned
                    else if (rect.right - work.right).abs() <= 50 && (other_rect.left - work.left).abs() <= 50 {
                        // Snap our left edge exactly to the other's right boundary!
                        if other_rect.right > work.left + 100 && other_rect.right < work.right - 100 {
                            rect.left = other_rect.right;
                        }
                    }
                }

                // Check vertically stacked alignment (overlap in X axis)
                let overlap_x = rect.right.min(other_rect.right) - rect.left.max(other_rect.left);
                if overlap_x > 100 {
                    // Case 3: Our target is top-aligned, and the other is bottom-aligned
                    if (rect.top - work.top).abs() <= 50 && (other_rect.bottom - work.bottom).abs() <= 50 {
                        // Snap our bottom edge exactly to the other's top boundary!
                        if other_rect.top > work.top + 100 && other_rect.top < work.bottom - 100 {
                            rect.bottom = other_rect.top;
                        }
                    }
                    // Case 4: Our target is bottom-aligned, and the other is top-aligned
                    else if (rect.bottom - work.bottom).abs() <= 50 && (other_rect.top - work.top).abs() <= 50 {
                        // Snap our top edge exactly to the other's bottom boundary!
                        if other_rect.bottom > work.top + 100 && other_rect.bottom < work.bottom - 100 {
                            rect.top = other_rect.bottom;
                        }
                    }
                }
            }
        }
    }
    rect
}
