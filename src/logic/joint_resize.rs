use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER, SWP_NOOWNERZORDER, SWP_NOCOPYBITS, SWP_NOSENDCHANGING};
use crate::logic::SNAPPED_WINDOWS;
use crate::logic::safety::get_window_shadow_margins;

pub fn perform_joint_resize(hwnd: HWND, new_x: i32, new_y: i32, new_w: i32, new_h: i32, prev_logical: RECT) {
    let margins = get_window_shadow_margins(hwnd);
    let new_logical = RECT {
        left: new_x + margins.left,
        top: new_y + margins.top,
        right: new_x + new_w - margins.right,
        bottom: new_y + new_h - margins.bottom,
    };

    let d_left = new_logical.left - prev_logical.left;
    let d_right = new_logical.right - prev_logical.right;
    let d_top = new_logical.top - prev_logical.top;
    let d_bottom = new_logical.bottom - prev_logical.bottom;

    let mut updates = Vec::new();

    if let Ok(guard) = SNAPPED_WINDOWS.lock() {
        for (&other_hwnd_val, &other_logical) in guard.iter() {
            if other_hwnd_val == hwnd.0 as isize {
                continue;
            }

            let mut new_other = other_logical;
            let mut changed = false;

            // 1. Right border / vertical division line movement
            if d_right != 0 {
                // Shift left borders of windows on the right side of this boundary
                if (other_logical.left - prev_logical.right).abs() <= 15 {
                    let val = other_logical.left + d_right;
                    if val < other_logical.right - 120 {
                        new_other.left = val;
                        changed = true;
                    }
                }
                // Shift right borders of windows on the left side of this boundary
                else if (other_logical.right - prev_logical.right).abs() <= 15 {
                    let val = other_logical.right + d_right;
                    if val > other_logical.left + 120 {
                        new_other.right = val;
                        changed = true;
                    }
                }
            }

            // 2. Left border / vertical division line movement
            if d_left != 0 {
                // Shift right borders of windows on the left side of this boundary
                if (other_logical.right - prev_logical.left).abs() <= 15 {
                    let val = other_logical.right + d_left;
                    if val > other_logical.left + 120 {
                        new_other.right = val;
                        changed = true;
                    }
                }
                // Shift left borders of windows on the right side of this boundary
                else if (other_logical.left - prev_logical.left).abs() <= 15 {
                    let val = other_logical.left + d_left;
                    if val < other_logical.right - 120 {
                        new_other.left = val;
                        changed = true;
                    }
                }
            }

            // 3. Bottom border / horizontal division line movement
            if d_bottom != 0 {
                // Shift top borders of windows below this boundary
                if (other_logical.top - prev_logical.bottom).abs() <= 15 {
                    let val = other_logical.top + d_bottom;
                    if val < other_logical.bottom - 120 {
                        new_other.top = val;
                        changed = true;
                    }
                }
                // Shift bottom borders of windows above this boundary
                else if (other_logical.bottom - prev_logical.bottom).abs() <= 15 {
                    let val = other_logical.bottom + d_bottom;
                    if val > other_logical.top + 120 {
                        new_other.bottom = val;
                        changed = true;
                    }
                }
            }

            // 4. Top border / horizontal division line movement
            if d_top != 0 {
                // Shift bottom borders of windows above this boundary
                if (other_logical.bottom - prev_logical.top).abs() <= 15 {
                    let val = other_logical.bottom + d_top;
                    if val > other_logical.top + 120 {
                        new_other.bottom = val;
                        changed = true;
                    }
                }
                // Shift top borders of windows below this boundary
                else if (other_logical.top - prev_logical.top).abs() <= 15 {
                    let val = other_logical.top + d_top;
                    if val < other_logical.bottom - 120 {
                        new_other.top = val;
                        changed = true;
                    }
                }
            }

            if changed {
                updates.push((other_hwnd_val, new_other));
            }
        }
    }

    // Reposition each updated neighbor window exactly once
    for &(other_hwnd_val, new_other) in updates.iter() {
        let other_margins = get_window_shadow_margins(HWND(other_hwnd_val as *mut _));
        let phys_left = new_other.left - other_margins.left;
        let phys_top = new_other.top - other_margins.top;
        let phys_w = (new_other.right - new_other.left) + other_margins.left + other_margins.right;
        let phys_h = (new_other.bottom - new_other.top) + other_margins.top + other_margins.bottom;

        unsafe {
            let _ = SetWindowPos(
                HWND(other_hwnd_val as *mut _),
                None,
                phys_left,
                phys_top,
                phys_w,
                phys_h,
                SWP_NOACTIVATE
                    | SWP_NOZORDER
                    | SWP_NOOWNERZORDER
                    | SWP_NOCOPYBITS
                    | SWP_NOSENDCHANGING,
            );
        }
    }

    // Apply all updates to other windows and update the active window's bounds
    if let Ok(mut guard) = SNAPPED_WINDOWS.lock() {
        guard.insert(hwnd.0 as isize, new_logical);
        for (u_hwnd, u_rect) in updates {
            guard.insert(u_hwnd, u_rect);
        }
    }
}
