use crate::input::{IS_OPERATION_ACTIVE, InputEvent, MouseAction, MouseButton};
use crate::ui::UiEvent;
use crossbeam_channel::{Receiver, Sender};
use std::sync::atomic::Ordering;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::UI::WindowsAndMessaging::{
    IsZoomed, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SWP_NOACTIVATE, SWP_NOCOPYBITS,
    SWP_NOSENDCHANGING, SWP_NOSIZE, SWP_NOZORDER, SWP_NOOWNERZORDER, SetForegroundWindow,
    SetWindowPos, ShowWindow, GetWindowRect,
};

pub mod types;
pub mod safety;
pub mod snapping;
pub mod joint_resize;

pub use types::*;
pub use safety::{is_valid_target_window, get_top_level_window_at};
pub use snapping::{calculate_balanced_rect, adjust_rect_for_adjacent_snapped_windows};
pub use joint_resize::perform_joint_resize;

/// Runs the Logic thread execution loop consumed from the raw mouse channels.
/// Manages states (Idle, Dragging, Resizing) and calculates coordinates in constant time.
pub fn run_logic_loop(
    rx: Receiver<InputEvent>,
    ui_tx: Sender<UiEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut mode = OperationMode::Idle;

    // Track frame timing to throttle move/resize updates to a smooth ~120 FPS,
    // protecting the OS from DWM/layout redraw congestion on high-polling gaming mice.
    let mut last_update_time = std::time::Instant::now();
    let mut stashed_event: Option<InputEvent> = None;

    // Track click state to recognize premium double click actions
    let mut last_click_time = std::time::Instant::now();
    let mut last_click_pos = (0, 0);

    println!("[OK] Logic thread execution loop active.");

    loop {
        let deadzone = crate::config::ATOMIC_DEADZONE_PIXELS.load(Ordering::Relaxed);
        let snap_threshold =
            crate::config::ATOMIC_SNAPPING_THRESHOLD_PIXELS.load(Ordering::Relaxed);

        let event_res = if let Some(ev) = stashed_event.take() {
            Ok(ev)
        } else {
            rx.recv()
        };

        let mut current_event = match event_res {
            Ok(ev) => ev,
            Err(_) => break, // Channel disconnected
        };

        // Coalesce consecutive MouseMove events to prevent "slow motion" trailing lag.
        if let InputEvent::MouseMove { .. } = current_event {
            while let Ok(next_event) = rx.try_recv() {
                match next_event {
                    InputEvent::MouseMove { .. } => {
                        current_event = next_event; // Keep only the latest coordinate
                    }
                    _ => {
                        stashed_event = Some(next_event); // Stash the button event for the next loop iteration
                        break;
                    }
                }
            }
        }

        match current_event {
            InputEvent::MouseMove {
                x,
                y,
                shift_pressed,
            } => {
                match mode {
                    OperationMode::Dragging {
                        hwnd,
                        start_cursor,
                        ref mut start_window_rect,
                        ref mut has_passed_deadzone,
                        ref mut is_zoning,
                    } => {
                        let dx = x - start_cursor.0;
                        let dy = y - start_cursor.1;

                        // 1. Enforce deadzone filter to prevent accidental jitter
                        if !*has_passed_deadzone {
                            if dx.abs() >= deadzone || dy.abs() >= deadzone {
                                *has_passed_deadzone = true;
                                IS_OPERATION_ACTIVE.store(true, Ordering::Relaxed);

                                // Restore original size only when we actually start dragging!
                                unsafe {
                                    if IsZoomed(hwnd).as_bool() {
                                        let _ = ShowWindow(hwnd, SW_RESTORE);
                                        let mut rect = RECT::default();
                                        if GetWindowRect(hwnd, &mut rect).is_ok() {
                                            let w = rect.right - rect.left;
                                            let h = rect.bottom - rect.top;

                                            let mut new_rect = RECT::default();
                                            new_rect.left = x - w / 2;
                                            new_rect.top = y - h / 2;
                                            new_rect.right = new_rect.left + w;
                                            new_rect.bottom = new_rect.top + h;
                                            *start_window_rect = new_rect;

                                            let _ = SetWindowPos(
                                                hwnd,
                                                None,
                                                new_rect.left,
                                                new_rect.top,
                                                w,
                                                h,
                                                SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOOWNERZORDER,
                                            );
                                        }
                                    } else {
                                        let mut found_pre_snap = None;
                                        if let Ok(guard) = PRE_SNAP_RECTS.lock() {
                                            if let Some(pre_rect) = guard.get(&(hwnd.0 as isize)) {
                                                found_pre_snap = Some(*pre_rect);
                                            }
                                        }
                                        if let Some(pre_rect) = found_pre_snap {
                                            let w = pre_rect.right - pre_rect.left;
                                            let h = pre_rect.bottom - pre_rect.top;

                                            let mut new_rect = RECT::default();
                                            new_rect.left = x - w / 2;
                                            new_rect.top = y - h / 2;
                                            new_rect.right = new_rect.left + w;
                                            new_rect.bottom = new_rect.top + h;
                                            *start_window_rect = new_rect;

                                            let _ = SetWindowPos(
                                                hwnd,
                                                None,
                                                new_rect.left,
                                                new_rect.top,
                                                w,
                                                h,
                                                SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOOWNERZORDER,
                                            );

                                            if let Ok(mut guard) = PRE_SNAP_RECTS.lock() {
                                                guard.remove(&(hwnd.0 as isize));
                                            }
                                            if let Ok(mut guard) = SNAPPED_WINDOWS.lock() {
                                                guard.remove(&(hwnd.0 as isize));
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if *has_passed_deadzone {
                            let now = std::time::Instant::now();
                            if now.duration_since(last_update_time).as_millis() >= 8 {
                                last_update_time = now;

                                let was_zoning = *is_zoning;
                                *is_zoning = shift_pressed && crate::config::ATOMIC_LAYOUTS_ENABLED.load(Ordering::Relaxed);

                                if *is_zoning {
                                    unsafe {
                                        let hmonitor =
                                            MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                                        if !hmonitor.0.is_null() {
                                            let mut monitor_info = MONITORINFO {
                                                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                                                ..Default::default()
                                            };
                                            if GetMonitorInfoW(hmonitor, &mut monitor_info)
                                                .as_bool()
                                            {
                                                let work = monitor_info.rcWork;
                                                let work_w = work.right - work.left;
                                                let work_h = work.bottom - work.top;

                                                let mut preview_rect = work;
                                                let x_rel = x - work.left;
                                                let y_rel = y - work.top;

                                                // Check if the cursor is over any already zoned window
                                                let mut target_zoned_window = None;
                                                let mut target_zoned_rect = RECT::default();
                                                if let Ok(guard) = SNAPPED_WINDOWS.lock() {
                                                    for (&other_hwnd_val, &other_rect) in guard.iter() {
                                                        if other_hwnd_val == hwnd.0 as isize {
                                                            continue;
                                                        }
                                                        if x >= other_rect.left && x <= other_rect.right
                                                            && y >= other_rect.top && y <= other_rect.bottom {
                                                            target_zoned_window = Some(HWND(other_hwnd_val as *mut _));
                                                            target_zoned_rect = other_rect;
                                                            break;
                                                        }
                                                    }
                                                }

                                                let split_enabled = crate::config::ATOMIC_SPLIT_ZONES_ENABLED.load(Ordering::Relaxed);
                                                let mut preview_rect_adjusted = preview_rect;

                                                if split_enabled && target_zoned_window.is_some() {
                                                    let _target_hwnd = target_zoned_window.unwrap();
                                                    let other_w = target_zoned_rect.right - target_zoned_rect.left;
                                                    let other_h = target_zoned_rect.bottom - target_zoned_rect.top;

                                                    if other_w > other_h {
                                                        // Suggest vertical split
                                                        let half_w = other_w / 2;
                                                        if x < target_zoned_rect.left + half_w {
                                                            preview_rect_adjusted.left = target_zoned_rect.left;
                                                            preview_rect_adjusted.right = target_zoned_rect.left + half_w;
                                                        } else {
                                                            preview_rect_adjusted.left = target_zoned_rect.left + half_w;
                                                            preview_rect_adjusted.right = target_zoned_rect.right;
                                                        }
                                                        preview_rect_adjusted.top = target_zoned_rect.top;
                                                        preview_rect_adjusted.bottom = target_zoned_rect.bottom;
                                                    } else {
                                                        // Suggest horizontal split
                                                        let half_h = other_h / 2;
                                                        if y < target_zoned_rect.top + half_h {
                                                            preview_rect_adjusted.top = target_zoned_rect.top;
                                                            preview_rect_adjusted.bottom = target_zoned_rect.top + half_h;
                                                        } else {
                                                            preview_rect_adjusted.top = target_zoned_rect.top + half_h;
                                                            preview_rect_adjusted.bottom = target_zoned_rect.bottom;
                                                        }
                                                        preview_rect_adjusted.left = target_zoned_rect.left;
                                                        preview_rect_adjusted.right = target_zoned_rect.right;
                                                    }
                                                } else {
                                                    // Standard monitor quadrants/thirds/halves logic
                                                    if y_rel <= 15 {
                                                        preview_rect = work;
                                                    } else {
                                                        if x_rel < work_w / 3 {
                                                            preview_rect.right = work.left + work_w / 3;
                                                        } else if x_rel > 2 * work_w / 3 {
                                                            preview_rect.left =
                                                                work.left + 2 * work_w / 3;
                                                        } else if x_rel < work_w / 2 {
                                                            preview_rect.right = work.left + work_w / 2;
                                                        } else {
                                                            preview_rect.left = work.left + work_w / 2;
                                                        }

                                                        if y_rel < work_h / 3 {
                                                            preview_rect.bottom = work.top + work_h / 2;
                                                        } else if y_rel > 2 * work_h / 3 {
                                                            preview_rect.top = work.top + work_h / 2;
                                                        }
                                                    }
                                                    preview_rect_adjusted = adjust_rect_for_adjacent_snapped_windows(preview_rect, hwnd, work);
                                                }

                                                let gap_pixels = crate::config::ATOMIC_GAP_PIXELS
                                                    .load(Ordering::Relaxed);
                                                let balanced_preview = calculate_balanced_rect(
                                                    preview_rect_adjusted,
                                                    work,
                                                    gap_pixels,
                                                );
                                                let _ = ui_tx.send(UiEvent::ShowPreview {
                                                    rect: balanced_preview,
                                                });
                                            }
                                        }
                                    }
                                } else if was_zoning {
                                    let _ = ui_tx.send(UiEvent::HidePreview);
                                }

                                let mut new_x = start_window_rect.left + dx;
                                let mut new_y = start_window_rect.top + dy;
                                let width = start_window_rect.right - start_window_rect.left;
                                let height = start_window_rect.bottom - start_window_rect.top;

                                unsafe {
                                    let hmonitor =
                                        MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                                    if !hmonitor.0.is_null() {
                                        let mut monitor_info = MONITORINFO {
                                            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                                            ..Default::default()
                                        };
                                        if GetMonitorInfoW(hmonitor, &mut monitor_info).as_bool() {
                                            let work = monitor_info.rcWork;

                                            if (new_x - work.left).abs() <= snap_threshold {
                                                new_x = work.left;
                                            }
                                            if ((new_x + width) - work.right).abs()
                                                <= snap_threshold
                                            {
                                                new_x = work.right - width;
                                            }
                                            if (new_y - work.top).abs() <= snap_threshold {
                                                new_y = work.top;
                                            }
                                            if ((new_y + height) - work.bottom).abs()
                                                <= snap_threshold
                                            {
                                                new_y = work.bottom - height;
                                            }
                                        }
                                    }

                                    let _ = SetWindowPos(
                                        hwnd,
                                        None,
                                        new_x,
                                        new_y,
                                        0,
                                        0,
                                        SWP_NOSIZE
                                            | SWP_NOACTIVATE
                                            | SWP_NOZORDER
                                            | SWP_NOOWNERZORDER,
                                    );
                                }
                            }
                        }
                    }
                    OperationMode::Resizing {
                        hwnd,
                        start_cursor,
                        start_window_rect,
                        active_region,
                        ref mut has_passed_deadzone,
                    } => {
                        let dx = x - start_cursor.0;
                        let dy = y - start_cursor.1;

                        if !*has_passed_deadzone {
                            if dx.abs() >= deadzone || dy.abs() >= deadzone {
                                *has_passed_deadzone = true;
                                IS_OPERATION_ACTIVE.store(true, Ordering::Relaxed);
                            }
                        }

                        if *has_passed_deadzone {
                            let now = std::time::Instant::now();
                            if now.duration_since(last_update_time).as_millis() >= 20 {
                                last_update_time = now;

                                if active_region == ActiveRegion::Center {
                                    let mut is_snapped = false;
                                    if let Ok(guard) = SNAPPED_WINDOWS.lock() {
                                        is_snapped = guard.contains_key(&(hwnd.0 as isize));
                                    }
                                    if is_snapped {
                                        continue;
                                    }
                                }

                                let mut new_x = start_window_rect.left;
                                let mut new_y = start_window_rect.top;
                                let mut new_w = start_window_rect.right - start_window_rect.left;
                                let mut new_h = start_window_rect.bottom - start_window_rect.top;

                                match active_region {
                                    ActiveRegion::TopLeft => {
                                        new_x = start_window_rect.left + dx;
                                        new_y = start_window_rect.top + dy;
                                        new_w =
                                            (start_window_rect.right - start_window_rect.left) - dx;
                                        new_h =
                                            (start_window_rect.bottom - start_window_rect.top) - dy;
                                    }
                                    ActiveRegion::Top => {
                                        new_y = start_window_rect.top + dy;
                                        new_h =
                                            (start_window_rect.bottom - start_window_rect.top) - dy;
                                    }
                                    ActiveRegion::TopRight => {
                                        new_y = start_window_rect.top + dy;
                                        new_w =
                                            (start_window_rect.right - start_window_rect.left) + dx;
                                        new_h =
                                            (start_window_rect.bottom - start_window_rect.top) - dy;
                                    }
                                    ActiveRegion::Left => {
                                        new_x = start_window_rect.left + dx;
                                        new_w =
                                            (start_window_rect.right - start_window_rect.left) - dx;
                                    }
                                    ActiveRegion::Right => {
                                        new_w =
                                            (start_window_rect.right - start_window_rect.left) + dx;
                                    }
                                    ActiveRegion::BottomLeft => {
                                        new_x = start_window_rect.left + dx;
                                        new_w =
                                            (start_window_rect.right - start_window_rect.left) - dx;
                                        new_h =
                                            (start_window_rect.bottom - start_window_rect.top) + dy;
                                    }
                                    ActiveRegion::Bottom => {
                                        new_h =
                                            (start_window_rect.bottom - start_window_rect.top) + dy;
                                    }
                                    ActiveRegion::BottomRight => {
                                        new_w =
                                            (start_window_rect.right - start_window_rect.left) + dx;
                                        new_h =
                                            (start_window_rect.bottom - start_window_rect.top) + dy;
                                    }
                                    ActiveRegion::Center => {
                                        new_x = start_window_rect.left - dx;
                                        new_y = start_window_rect.top - dy;
                                        new_w = (start_window_rect.right - start_window_rect.left)
                                            + 2 * dx;
                                        new_h = (start_window_rect.bottom - start_window_rect.top)
                                            + 2 * dy;
                                    }
                                }

                                if new_w < 120 {
                                    new_w = 120;
                                }
                                if new_h < 120 {
                                    new_h = 120;
                                }

                                unsafe {
                                    let _ = SetWindowPos(
                                        hwnd,
                                        None,
                                        new_x,
                                        new_y,
                                        new_w,
                                        new_h,
                                        SWP_NOACTIVATE
                                            | SWP_NOZORDER
                                            | SWP_NOOWNERZORDER
                                            | SWP_NOCOPYBITS
                                            | SWP_NOSENDCHANGING,
                                    );

                                    // Joint border dynamic tiling updates
                                    let mut is_snapped = false;
                                    let mut prev_logical = RECT::default();
                                    if let Ok(guard) = SNAPPED_WINDOWS.lock() {
                                        if let Some(r) = guard.get(&(hwnd.0 as isize)) {
                                            is_snapped = true;
                                            prev_logical = *r;
                                        }
                                    }

                                    if is_snapped {
                                        perform_joint_resize(hwnd, new_x, new_y, new_w, new_h, prev_logical);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            InputEvent::MouseButton {
                button,
                action,
                x,
                y,
            } => {
                match action {
                    MouseAction::Press => {
                        crate::config::reload_config();

                        if mode == OperationMode::Idle {
                            let now = std::time::Instant::now();
                            let elapsed = now.duration_since(last_click_time).as_millis();
                            let dist_sq =
                                (x - last_click_pos.0).pow(2) + (y - last_click_pos.1).pow(2);

                            if crate::config::ATOMIC_GESTURES_ENABLED.load(Ordering::Relaxed) && elapsed <= 500 && dist_sq <= 64 {
                                if button == MouseButton::Left {
                                    let hwnd = get_top_level_window_at(x, y);
                                    if !hwnd.0.is_null() {
                                        unsafe {
                                            let _ = SetForegroundWindow(hwnd);
                                            if IsZoomed(hwnd).as_bool() {
                                                let _ = ShowWindow(hwnd, SW_RESTORE);
                                            } else {
                                                let _ = ShowWindow(hwnd, SW_MAXIMIZE);
                                            }
                                        }
                                    }
                                    last_click_time = now;
                                    last_click_pos = (x, y);
                                    continue;
                                } else if button == MouseButton::Right {
                                    let hwnd = get_top_level_window_at(x, y);
                                    if !hwnd.0.is_null() {
                                        unsafe {
                                            let _ = ShowWindow(hwnd, SW_MINIMIZE);
                                        }
                                    }
                                    last_click_time = now;
                                    last_click_pos = (x, y);
                                    continue;
                                }
                            }

                            last_click_time = now;
                            last_click_pos = (x, y);

                            let hwnd = get_top_level_window_at(x, y);
                            if !hwnd.0.is_null() {
                                let mut rect = RECT::default();
                                unsafe {
                                    if GetWindowRect(hwnd, &mut rect).is_ok() {
                                        let _ = SetForegroundWindow(hwnd);

                                        if button == MouseButton::Left {
                                            mode = OperationMode::Dragging {
                                                hwnd,
                                                start_cursor: (x, y),
                                                start_window_rect: rect,
                                                has_passed_deadzone: false,
                                                is_zoning: false,
                                            };
                                        } else {
                                            let w = rect.right - rect.left;
                                            let h = rect.bottom - rect.top;
                                            let x_rel = x - rect.left;
                                            let y_rel = y - rect.top;

                                            let region = if x_rel < w / 3 {
                                                if y_rel < h / 3 {
                                                    ActiveRegion::TopLeft
                                                } else if y_rel > 2 * h / 3 {
                                                    ActiveRegion::BottomLeft
                                                } else {
                                                    ActiveRegion::Left
                                                }
                                            } else if x_rel > 2 * w / 3 {
                                                if y_rel < h / 3 {
                                                    ActiveRegion::TopRight
                                                } else if y_rel > 2 * h / 3 {
                                                    ActiveRegion::BottomRight
                                                } else {
                                                    ActiveRegion::Right
                                                }
                                            } else {
                                                if y_rel < h / 3 {
                                                    ActiveRegion::Top
                                                } else if y_rel > 2 * h / 3 {
                                                    ActiveRegion::Bottom
                                                } else {
                                                    ActiveRegion::Center
                                                }
                                            };

                                            mode = OperationMode::Resizing {
                                                hwnd,
                                                start_cursor: (x, y),
                                                start_window_rect: rect,
                                                active_region: region,
                                                has_passed_deadzone: false,
                                            };
                                        }
                                    }
                                }
                            }
                        }
                    }
                    MouseAction::Release => {
                        IS_OPERATION_ACTIVE.store(false, Ordering::Relaxed);

                        if let OperationMode::Dragging {
                            is_zoning,
                            hwnd,
                            start_window_rect,
                            ..
                        } = mode
                        {
                            unsafe {
                                let hmonitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                                if !hmonitor.0.is_null() {
                                    let mut monitor_info = MONITORINFO {
                                        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                                        ..Default::default()
                                    };
                                    if GetMonitorInfoW(hmonitor, &mut monitor_info).as_bool() {
                                        let work = monitor_info.rcWork;
                                        let y_rel = y - work.top;

                                        if y_rel <= 15 {
                                            if is_zoning {
                                                let _ = ui_tx.send(UiEvent::HidePreview);
                                            }
                                            let _ = ShowWindow(hwnd, SW_MAXIMIZE);
                                        } else if is_zoning {
                                            let _ = ui_tx.send(UiEvent::HidePreview);

                                            let work_w = work.right - work.left;
                                            let work_h = work.bottom - work.top;

                                            // 1. Check if the cursor is over any already zoned window
                                            let mut target_zoned_window = None;
                                            let mut target_zoned_rect = RECT::default();
                                            if let Ok(guard) = SNAPPED_WINDOWS.lock() {
                                                for (&other_hwnd_val, &other_rect) in guard.iter() {
                                                    if other_hwnd_val == hwnd.0 as isize {
                                                        continue;
                                                    }
                                                    if x >= other_rect.left && x <= other_rect.right
                                                        && y >= other_rect.top && y <= other_rect.bottom {
                                                        target_zoned_window = Some(HWND(other_hwnd_val as *mut _));
                                                        target_zoned_rect = other_rect;
                                                        break;
                                                    }
                                                }
                                            }

                                            let split_enabled = crate::config::ATOMIC_SPLIT_ZONES_ENABLED.load(Ordering::Relaxed);
                                            if split_enabled && target_zoned_window.is_some() {
                                                let target_hwnd = target_zoned_window.unwrap();
                                                let other_w = target_zoned_rect.right - target_zoned_rect.left;
                                                let other_h = target_zoned_rect.bottom - target_zoned_rect.top;

                                                let mut snap_rect_adjusted = target_zoned_rect;
                                                let mut other_new_rect = target_zoned_rect;

                                                if other_w > other_h {
                                                    // Vertical split
                                                    let half_w = other_w / 2;
                                                    if x < target_zoned_rect.left + half_w {
                                                        snap_rect_adjusted.right = target_zoned_rect.left + half_w;
                                                        other_new_rect.left = target_zoned_rect.left + half_w;
                                                    } else {
                                                        snap_rect_adjusted.left = target_zoned_rect.left + half_w;
                                                        other_new_rect.right = target_zoned_rect.left + half_w;
                                                    }
                                                } else {
                                                    // Horizontal split
                                                    let half_h = other_h / 2;
                                                    if y < target_zoned_rect.top + half_h {
                                                        snap_rect_adjusted.bottom = target_zoned_rect.top + half_h;
                                                        other_new_rect.top = target_zoned_rect.top + half_h;
                                                    } else {
                                                        snap_rect_adjusted.top = target_zoned_rect.top + half_h;
                                                        other_new_rect.bottom = target_zoned_rect.top + half_h;
                                                    }
                                                }

                                                if let Ok(mut guard) = PRE_SNAP_RECTS.lock() {
                                                    guard.insert(hwnd.0 as isize, start_window_rect);
                                                }

                                                // Update and reposition our dragged window
                                                let margins = safety::get_window_shadow_margins(hwnd);
                                                let snap_x = snap_rect_adjusted.left - margins.left;
                                                let snap_y = snap_rect_adjusted.top - margins.top;
                                                let snap_w = (snap_rect_adjusted.right - snap_rect_adjusted.left)
                                                    + margins.left
                                                    + margins.right;
                                                let snap_h = (snap_rect_adjusted.bottom - snap_rect_adjusted.top)
                                                    + margins.top
                                                    + margins.bottom;

                                                let _ = SetWindowPos(
                                                    hwnd,
                                                    None,
                                                    snap_x,
                                                    snap_y,
                                                    snap_w,
                                                    snap_h,
                                                    SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOOWNERZORDER,
                                                );

                                                // Update and reposition the split zoned window
                                                let other_margins = safety::get_window_shadow_margins(target_hwnd);
                                                let other_x = other_new_rect.left - other_margins.left;
                                                let other_y = other_new_rect.top - other_margins.top;
                                                let other_w_phys = (other_new_rect.right - other_new_rect.left)
                                                    + other_margins.left
                                                    + other_margins.right;
                                                let other_h_phys = (other_new_rect.bottom - other_new_rect.top)
                                                    + other_margins.top
                                                    + other_margins.bottom;

                                                let _ = SetWindowPos(
                                                    target_hwnd,
                                                    None,
                                                    other_x,
                                                    other_y,
                                                    other_w_phys,
                                                    other_h_phys,
                                                    SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOOWNERZORDER,
                                                );

                                                // Save both windows in the snapping cache!
                                                if let Ok(mut guard) = SNAPPED_WINDOWS.lock() {
                                                    guard.insert(hwnd.0 as isize, snap_rect_adjusted);
                                                    guard.insert(target_hwnd.0 as isize, other_new_rect);
                                                }
                                            } else {
                                                // Standard monitor quadrants/thirds/halves logic
                                                let mut snap_rect = work;
                                                let x_rel = x - work.left;

                                                if x_rel < work_w / 3 {
                                                    snap_rect.right = work.left + work_w / 3;
                                                } else if x_rel > 2 * work_w / 3 {
                                                    snap_rect.left = work.left + 2 * work_w / 3;
                                                } else if x_rel < work_w / 2 {
                                                    snap_rect.right = work.left + work_w / 2;
                                                } else {
                                                    snap_rect.left = work.left + work_w / 2;
                                                }

                                                if y_rel < work_h / 3 {
                                                    snap_rect.bottom = work.top + work_h / 2;
                                                } else if y_rel > 2 * work_h / 3 {
                                                    snap_rect.top = work.top + work_h / 2;
                                                }

                                                let snap_rect_adjusted = adjust_rect_for_adjacent_snapped_windows(snap_rect, hwnd, work);

                                                if let Ok(mut guard) = PRE_SNAP_RECTS.lock() {
                                                    guard.insert(hwnd.0 as isize, start_window_rect);
                                                }

                                                let margins = safety::get_window_shadow_margins(hwnd);
                                                let snap_x = snap_rect_adjusted.left - margins.left;
                                                let snap_y = snap_rect_adjusted.top - margins.top;
                                                let snap_w = (snap_rect_adjusted.right - snap_rect_adjusted.left)
                                                    + margins.left
                                                    + margins.right;
                                                let snap_h = (snap_rect_adjusted.bottom - snap_rect_adjusted.top)
                                                    + margins.top
                                                    + margins.bottom;

                                                let _ = SetWindowPos(
                                                    hwnd,
                                                    None,
                                                    snap_x,
                                                    snap_y,
                                                    snap_w,
                                                    snap_h,
                                                    SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOOWNERZORDER,
                                                );

                                                if let Ok(mut guard) = SNAPPED_WINDOWS.lock() {
                                                    guard.insert(hwnd.0 as isize, snap_rect_adjusted);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        mode = OperationMode::Idle;
                    }
                }
            }
        }
    }

    Ok(())
}
