use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM, COLORREF};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush,
    DeleteDC, DeleteObject, EndPaint, SelectObject, CreateFontW, SetTextColor, PAINTSTRUCT,
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, FW_NORMAL,
    OUT_DEFAULT_PRECIS, InvalidateRect,
};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CS_HREDRAW, CS_VREDRAW, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
    DestroyMenu, DispatchMessageW, ES_AUTOVSCROLL, ES_CENTER, ES_MULTILINE, ES_NUMBER,
    GetClientRect, GetCursorPos, GetMessageW, HMENU, IDC_ARROW, IDI_APPLICATION, LoadCursorW,
    LoadIconW, MF_SEPARATOR, MF_STRING, MSG, PostQuitMessage, RegisterClassW, SW_HIDE, SW_SHOW,
    SendMessageW, SetForegroundWindow, ShowWindow, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    TrackPopupMenu, TranslateMessage, WINDOW_STYLE, WM_CLOSE, WM_CREATE, WM_DESTROY,
    WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_PAINT, WM_RBUTTONUP, WM_SETFONT, WM_USER,
    WNDCLASSW, WS_CAPTION, WS_CHILD, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU,
};
use windows::core::{PCWSTR, w};

pub mod types;
pub mod actions;
pub mod state;
pub mod controls;
pub mod paint;

pub use state::{
    SafeHwnd, SafeHbrush, HoveredControl, DashboardState, STATE, SETTINGS_HWND,
    HWND_BLACKLIST_EDIT, HWND_DEADZONE, HWND_THRESHOLD, HWND_GAP, HWND_RADIUS, HWND_OPACITY,
    INPUT_BG_BRUSH, commit_settings_action,
};
pub use controls::update_controls_visibility;
pub use paint::{draw_text_str, draw_dashboard_fluent};

const WM_TRAY_ICON: u32 = WM_USER + 100;

pub fn spawn_settings_thread() {
    std::thread::spawn(|| {
        if let Err(e) = run_settings_gui() {
            eprintln!("[ERROR] Settings thread crashed: {:?}", e);
        }
    });
}

fn run_settings_gui() -> Result<(), Box<dyn std::error::Error>> {
    let instance = unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleW(None)? };
    let class_name = w!("WingripSettingsFluent");

    unsafe {
        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(settings_wnd_proc),
            hInstance: instance.into(),
            hIcon: LoadIconW(None, IDI_APPLICATION)?,
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            lpszClassName: class_name,
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH(16 as *mut _),
            ..Default::default()
        };

        if RegisterClassW(&wnd_class) == 0 {
            return Err("Failed to register Settings window class".into());
        }

        // Elegant fixed widescreen size dashboard
        let hwnd = CreateWindowExW(
            windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
            class_name,
            w!("wingrip settings"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            200,
            200,
            740,
            560,
            HWND::default(),
            HMENU::default(),
            instance,
            None,
        )?;

        if hwnd.0.is_null() {
            return Err("Failed to create Settings window".into());
        }

        // Apply Immersive Dark Mode and custom brand colors to the native title bar (Win11+)
        let dark_mode = windows::Win32::Foundation::TRUE;
        let _ = windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(
            hwnd,
            windows::Win32::Graphics::Dwm::DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark_mode as *const _ as *const _,
            std::mem::size_of::<windows::Win32::Foundation::BOOL>() as u32,
        );

        // Match title bar background to deep slate-purple BGR: 0x0022181C
        let caption_color: u32 = 0x0022181C;
        let _ = windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(
            hwnd,
            windows::Win32::Graphics::Dwm::DWMWA_CAPTION_COLOR,
            &caption_color as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        );

        // Color title bar border to premium neon lavender BGR: 0x00DC96B4
        let border_color: u32 = 0x00DC96B4;
        let _ = windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(
            hwnd,
            windows::Win32::Graphics::Dwm::DWMWA_BORDER_COLOR,
            &border_color as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        );

        // Customize title bar text color to crisp white BGR: 0x00FFFFFF
        let text_color: u32 = 0x00FFFFFF;
        let _ = windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(
            hwnd,
            windows::Win32::Graphics::Dwm::DWMWA_TEXT_COLOR,
            &text_color as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        );

        if let Ok(mut guard) = SETTINGS_HWND.lock() {
            *guard = Some(SafeHwnd(hwnd));
        }

        // Add System Tray Icon
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAY_ICON,
            hIcon: LoadIconW(None, IDI_APPLICATION)?,
            ..Default::default()
        };
        let tooltip_chars = "wingrip".encode_utf16().collect::<Vec<u16>>();
        let len = tooltip_chars.len().min(127);
        nid.szTip[..len].copy_from_slice(&tooltip_chars[..len]);

        let _ = Shell_NotifyIconW(NIM_ADD, &nid);

        // Windows message pump
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }

    Ok(())
}

unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_CREATE => {
                let state = STATE.lock().unwrap();
                let instance =
                    windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap();
                let hfont = CreateFontW(
                    16,
                    0,
                    0,
                    0,
                    FW_NORMAL.0 as i32,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET.0 as u32,
                    OUT_DEFAULT_PRECIS.0 as u32,
                    CLIP_DEFAULT_PRECIS.0 as u32,
                    CLEARTYPE_QUALITY.0 as u32,
                    DEFAULT_PITCH.0 as u32,
                    w!("Segoe UI"),
                );

                // Create the global solid brush for dark edit boxes colorized via WM_CTLCOLOREDIT
                *INPUT_BG_BRUSH.lock().unwrap() =
                    Some(SafeHbrush(CreateSolidBrush(COLORREF(0x001B1214))));

                // Helper to quickly spawn a styled numeric input box
                let create_numeric_input = |val: i32, y_pos: i32| -> HWND {
                    let val_w = val
                        .to_string()
                        .encode_utf16()
                        .chain(std::iter::once(0))
                        .collect::<Vec<u16>>();
                    let edit_hwnd = CreateWindowExW(
                        windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                        w!("EDIT"),
                        PCWSTR::from_raw(val_w.as_ptr()),
                        WINDOW_STYLE(WS_CHILD.0 | ES_NUMBER as u32 | ES_CENTER as u32),
                        551,
                        y_pos + 2,
                        108,
                        24,
                        hwnd,
                        HMENU::default(),
                        instance,
                        None,
                    )
                    .unwrap();
                    SendMessageW(edit_hwnd, WM_SETFONT, WPARAM(hfont.0 as usize), LPARAM(1));
                    edit_hwnd
                };

                // Tab 0 Numeric Inputs
                let ed_deadzone = create_numeric_input(state.deadzone, 138);
                *HWND_DEADZONE.lock().unwrap() = Some(SafeHwnd(ed_deadzone));

                let ed_threshold = create_numeric_input(state.threshold, 218);
                *HWND_THRESHOLD.lock().unwrap() = Some(SafeHwnd(ed_threshold));

                // Tab 1 Numeric Inputs
                let ed_gap = create_numeric_input(state.gap, 138);
                *HWND_GAP.lock().unwrap() = Some(SafeHwnd(ed_gap));

                let ed_radius = create_numeric_input(state.radius, 218);
                *HWND_RADIUS.lock().unwrap() = Some(SafeHwnd(ed_radius));

                let ed_opacity = create_numeric_input(state.opacity as i32, 298);
                *HWND_OPACITY.lock().unwrap() = Some(SafeHwnd(ed_opacity));

                // Creates a hidden multi-line edit box for Tab 2 (Blacklist) which we only show when Tab 2 is active
                let blacklist_w = state
                    .blacklist_text
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect::<Vec<u16>>();
                let edit = CreateWindowExW(
                    windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                    w!("EDIT"),
                    PCWSTR::from_raw(blacklist_w.as_ptr()),
                    WINDOW_STYLE(WS_CHILD.0 | ES_MULTILINE as u32 | ES_AUTOVSCROLL as u32),
                    242,
                    162,
                    436,
                    236,
                    hwnd,
                    HMENU::default(),
                    instance,
                    None,
                )
                .unwrap();
                SendMessageW(edit, WM_SETFONT, WPARAM(hfont.0 as usize), LPARAM(1));

                *HWND_BLACKLIST_EDIT.lock().unwrap() = Some(SafeHwnd(edit));

                // Instantly sync their initial visibilities
                update_controls_visibility(0);
            }

            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);

                // Client Rect sizing
                let mut rect = RECT::default();
                let _ = GetClientRect(hwnd, &mut rect);
                let w = rect.right - rect.left;
                let h = rect.bottom - rect.top;

                // Flicker-free double buffering
                let mem_dc = CreateCompatibleDC(hdc);
                let mem_bmp = CreateCompatibleBitmap(hdc, w, h);
                let old_bmp = SelectObject(mem_dc, mem_bmp);

                // Draw gorgeous dashboard background
                draw_dashboard_fluent(mem_dc, w, h);

                // Blit buffer to screen
                let _ = BitBlt(
                    hdc,
                    0,
                    0,
                    w,
                    h,
                    mem_dc,
                    0,
                    0,
                    windows::Win32::Graphics::Gdi::SRCCOPY,
                );

                // Cleanup double buffer handles
                let _ = SelectObject(mem_dc, old_bmp);
                let _ = DeleteObject(mem_bmp);
                let _ = DeleteDC(mem_dc);

                let _ = EndPaint(hwnd, &ps);
            }

            windows::Win32::UI::WindowsAndMessaging::WM_CTLCOLOREDIT
            | windows::Win32::UI::WindowsAndMessaging::WM_CTLCOLORSTATIC => {
                let hdc = windows::Win32::Graphics::Gdi::HDC(wparam.0 as *mut _);
                SetTextColor(hdc, COLORREF(0x00FFFFFF));
                windows::Win32::Graphics::Gdi::SetBkColor(hdc, COLORREF(0x001B1214));
                if let Some(brush) = *INPUT_BG_BRUSH.lock().unwrap() {
                    return LRESULT(brush.0.0 as isize);
                }
                return LRESULT(0);
            }

            WM_MOUSEMOVE => {
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

                let mut new_hover = HoveredControl::None;
                let state = STATE.lock().unwrap();

                // 1. Sidebar Tabs Hitboxes
                if x < 200 {
                    if y >= 60 && y < 100 {
                        new_hover = HoveredControl::SidebarTab(0);
                    } else if y >= 105 && y < 145 {
                        new_hover = HoveredControl::SidebarTab(1);
                    } else if y >= 150 && y < 190 {
                        new_hover = HoveredControl::SidebarTab(2);
                    }
                } else {
                    // Save Button (Y = 460, Width = 440, Height = 36)
                    if x >= 240 && x < 680 && y >= 460 && y < 496 {
                        new_hover = HoveredControl::SaveButton;
                    }
                }

                // Trigger repaint on hover changes
                if new_hover != state.hovered {
                    drop(state);
                    if let Ok(mut guard) = STATE.lock() {
                        guard.hovered = new_hover;
                    }
                    let _ = InvalidateRect(hwnd, None, false);
                }
            }

            WM_LBUTTONDOWN => {
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

                let mut state = STATE.lock().unwrap();

                // 1. Sidebar tab switching
                if x < 200 {
                    let mut tab_clicked = None;
                    if y >= 60 && y < 100 {
                        tab_clicked = Some(0);
                    } else if y >= 105 && y < 145 {
                        tab_clicked = Some(1);
                    } else if y >= 150 && y < 190 {
                        tab_clicked = Some(2);
                    }

                    if let Some(tab) = tab_clicked {
                        state.active_tab = tab;
                        update_controls_visibility(tab);
                    }
                } else {
                    // 2. Main Area adjustment controllers (Toggles only)
                    match state.active_tab {
                        0 => {
                            // Start with Windows (Toggle Switch)
                            if y >= 280 && y < 344 {
                                state.startup_enabled = !state.startup_enabled;
                            }
                            // Shortcut Gestures (Toggle Switch)
                            else if y >= 360 && y < 424 {
                                state.gestures_enabled = !state.gestures_enabled;
                            }
                        }
                        1 => {
                            // Snapping Layouts (Toggle Switch)
                            if y >= 360 && y < 424 {
                                state.layouts_enabled = !state.layouts_enabled;
                            }
                        }
                        _ => {}
                    }

                    // 3. Save Button clicked
                    if x >= 240 && x < 680 && y >= 460 && y < 496 {
                        drop(state);
                        if let Err(e) = commit_settings_action(hwnd) {
                            eprintln!("[ERROR] Failed to save settings: {:?}", e);
                        }
                        return LRESULT(0);
                    }
                }

                drop(state);
                let _ = InvalidateRect(hwnd, None, false);
            }

            WM_TRAY_ICON => {
                let event = lparam.0 as u32;
                if event == WM_RBUTTONUP {
                    let mut pt = POINT::default();
                    if GetCursorPos(&mut pt).is_ok() {
                        let _ = SetForegroundWindow(hwnd);
                        let hmenu = CreatePopupMenu().unwrap();
                        let _ = AppendMenuW(hmenu, MF_STRING, 1, w!("Settings Panel"));
                        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);
                        let _ = AppendMenuW(hmenu, MF_STRING, 2, w!("Exit wingrip"));

                        let cmd = TrackPopupMenu(
                            hmenu,
                            TPM_RETURNCMD | TPM_LEFTALIGN | TPM_RIGHTBUTTON,
                            pt.x,
                            pt.y,
                            0,
                            hwnd,
                            None,
                        );
                        let _ = DestroyMenu(hmenu);

                        if cmd.0 == 1 {
                            let _ = ShowWindow(hwnd, SW_SHOW);
                            let _ = SetForegroundWindow(hwnd);
                        } else if cmd.0 == 2 {
                            PostQuitMessage(0);
                            std::process::exit(0);
                        }
                    }
                } else if event == WM_LBUTTONDBLCLK {
                    if windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd).as_bool() {
                        let _ = ShowWindow(hwnd, SW_HIDE);
                    } else {
                        let _ = ShowWindow(hwnd, SW_SHOW);
                        let _ = SetForegroundWindow(hwnd);
                    }
                }
            }

            WM_CLOSE => {
                let _ = ShowWindow(hwnd, SW_HIDE);
                return LRESULT(0);
            }

            WM_DESTROY => {
                if let Ok(mut guard) = INPUT_BG_BRUSH.lock() {
                    if let Some(brush) = guard.take() {
                        let _ = DeleteObject(brush.0);
                    }
                }
                PostQuitMessage(0);
                return LRESULT(0);
            }

            _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
    LRESULT(0)
}
