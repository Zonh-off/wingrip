use windows::Win32::Foundation::{COLORREF, RECT};
use windows::Win32::Graphics::Gdi::{
    CreatePen, CreateSolidBrush, DeleteObject, DrawTextW, RoundRect, SelectObject, SetBkMode,
    SetTextColor, HDC, PS_SOLID, TRANSPARENT, FW_BOLD, FW_NORMAL, DEFAULT_CHARSET,
    OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, DEFAULT_PITCH, CreateFontW,
    DT_SINGLELINE, DT_VCENTER, DT_LEFT, DT_CENTER, FillRect,
};
use windows::core::w;
use crate::settings::state::{STATE, HoveredControl};

pub fn draw_text_str(
    hdc: HDC,
    text: &str,
    rect: &mut RECT,
    format: windows::Win32::Graphics::Gdi::DRAW_TEXT_FORMAT,
) {
    unsafe {
        let mut wide: Vec<u16> = text.encode_utf16().collect();
        let _ = DrawTextW(hdc, &mut wide, rect, format);
    }
}

// Gorgeous Fluent Design rendering pipeline via GDI Double Buffering
pub fn draw_dashboard_fluent(hdc: HDC, w: i32, h: i32) {
    unsafe {
        // High-end warm slate and premium dark lavender palette
        let bg_color = COLORREF(0x0022181C); // Warm slate background
        let sidebar_color = COLORREF(0x001B1214); // Sleek deep sidebar
        let sidebar_border = COLORREF(0x00281F22); // Fine edge separator
        let card_bg = COLORREF(0x002A2024); // Card background
        let card_border = COLORREF(0x003B2E33); // Subtle card border
        let text_white = COLORREF(0x00FFFFFF);
        let text_gray = COLORREF(0x00B0979E); // Muted lavender-grey
        let text_accent = COLORREF(0x00DC96B4); // Active neon lavender accent
        let hover_bg = COLORREF(0x0033282C);

        // Setup background brushes
        let bg_brush = CreateSolidBrush(bg_color);
        let sidebar_brush = CreateSolidBrush(sidebar_color);
        let card_brush = CreateSolidBrush(card_bg);
        let hover_brush = CreateSolidBrush(hover_bg);

        // Draw Window frame
        let rect_bg = RECT {
            left: 0,
            top: 0,
            right: w,
            bottom: h,
        };
        FillRect(hdc, &rect_bg, bg_brush);

        // Draw Left Sidebar Panel
        let rect_sidebar = RECT {
            left: 0,
            top: 0,
            right: 200,
            bottom: h,
        };
        FillRect(hdc, &rect_sidebar, sidebar_brush);

        // Draw Sidebar right-side separator
        let border_pen = CreatePen(PS_SOLID, 1, sidebar_border);
        let _old_pen = SelectObject(hdc, border_pen);
        let _ = windows::Win32::Graphics::Gdi::MoveToEx(hdc, 200, 0, None);
        let _ = windows::Win32::Graphics::Gdi::LineTo(hdc, 200, h);

        // Initialize premium Fluent Segoe UI Typography
        let font_title = CreateFontW(
            22,
            0,
            0,
            0,
            FW_BOLD.0 as i32,
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
        let font_card_title = CreateFontW(
            15,
            0,
            0,
            0,
            FW_BOLD.0 as i32,
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
        let font_body = CreateFontW(
            14,
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
        let font_desc = CreateFontW(
            12,
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

        // ----------------------------------------------------
        // DRAW SIDEBAR TABS
        // ----------------------------------------------------
        let state = STATE.lock().unwrap();
        let tabs = ["General", "Snapping", "Blacklist"];

        for i in 0..3 {
            let y = 60 + (i as i32 * 45);
            let tab_rect = RECT {
                left: 10,
                top: y,
                right: 190,
                bottom: y + 40,
            };

            // Draw hover / selection card
            if state.active_tab == i {
                let active_pen = CreatePen(PS_SOLID, 1, card_border);
                let old_p = SelectObject(hdc, active_pen);
                let _ = SelectObject(hdc, card_brush);
                let _ = RoundRect(
                    hdc,
                    tab_rect.left,
                    tab_rect.top,
                    tab_rect.right,
                    tab_rect.bottom,
                    8,
                    8,
                );
                let _ = SelectObject(hdc, old_p);
                let _ = DeleteObject(active_pen);

                // Draw active indicator marker
                let marker_brush = CreateSolidBrush(text_accent);
                let marker_rect = RECT {
                    left: 14,
                    top: y + 10,
                    right: 18,
                    bottom: y + 30,
                };
                FillRect(hdc, &marker_rect, marker_brush);
                let _ = DeleteObject(marker_brush);
            } else if state.hovered == HoveredControl::SidebarTab(i) {
                let hover_pen = CreatePen(PS_SOLID, 1, sidebar_border);
                let old_p = SelectObject(hdc, hover_pen);
                let _ = SelectObject(hdc, hover_brush);
                let _ = RoundRect(
                    hdc,
                    tab_rect.left,
                    tab_rect.top,
                    tab_rect.right,
                    tab_rect.bottom,
                    8,
                    8,
                );
                let _ = SelectObject(hdc, old_p);
                let _ = DeleteObject(hover_pen);
            }

            // Draw tab labels
            let _ = SetBkMode(hdc, TRANSPARENT);
            SetTextColor(
                hdc,
                if state.active_tab == i {
                    text_white
                } else {
                    text_gray
                },
            );
            let old_f = SelectObject(hdc, font_card_title);
            let mut text_rect = RECT {
                left: 28,
                top: y,
                right: 180,
                bottom: y + 40,
            };
            draw_text_str(
                hdc,
                tabs[i],
                &mut text_rect,
                DT_SINGLELINE | DT_VCENTER | DT_LEFT,
            );
            let _ = SelectObject(hdc, old_f);
        }

        // ----------------------------------------------------
        // DRAW MAIN PANEL AREA
        // ----------------------------------------------------
        let old_f_t = SelectObject(hdc, font_title);
        let _ = SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, text_white);

        // Draw Header
        let mut header_rect = RECT {
            left: 240,
            top: 25,
            right: 700,
            bottom: 55,
        };
        match state.active_tab {
            0 => {
                draw_text_str(
                    hdc,
                    "General Settings",
                    &mut header_rect,
                    DT_SINGLELINE | DT_VCENTER | DT_LEFT,
                );
                let _ = SelectObject(hdc, font_desc);
                SetTextColor(hdc, text_gray);
                let mut sub_rect = RECT {
                    left: 240,
                    top: 55,
                    right: 700,
                    bottom: 75,
                };
                draw_text_str(
                    hdc,
                    "Configure low-level snapping parameters and window deadzones.",
                    &mut sub_rect,
                    DT_SINGLELINE | DT_VCENTER | DT_LEFT,
                );
            }
            1 => {
                draw_text_str(
                    hdc,
                    "Snapping Layouts",
                    &mut header_rect,
                    DT_SINGLELINE | DT_VCENTER | DT_LEFT,
                );
                let _ = SelectObject(hdc, font_desc);
                SetTextColor(hdc, text_gray);
                let mut sub_rect = RECT {
                    left: 240,
                    top: 55,
                    right: 700,
                    bottom: 75,
                };
                draw_text_str(
                    hdc,
                    "Customize visual snapping overlays, tiling bounds, colors and gaps.",
                    &mut sub_rect,
                    DT_SINGLELINE | DT_VCENTER | DT_LEFT,
                );
            }
            _ => {
                draw_text_str(
                    hdc,
                    "Game Blacklist",
                    &mut header_rect,
                    DT_SINGLELINE | DT_VCENTER | DT_LEFT,
                );
                let _ = SelectObject(hdc, font_desc);
                SetTextColor(hdc, text_gray);
                let mut sub_rect = RECT {
                    left: 240,
                    top: 55,
                    right: 700,
                    bottom: 75,
                };
                draw_text_str(
                    hdc,
                    "Specify processes that should bypass mouse hooks (one executable per line).",
                    &mut sub_rect,
                    DT_SINGLELINE | DT_VCENTER | DT_LEFT,
                );
            }
        }

        // ----------------------------------------------------
        // VISUAL CARD DRAWING HELPERS
        // ----------------------------------------------------

        // 1. Draw Gorgeous Custom Input Card (leaving the right side empty for the native Edit control)
        let draw_input_card = |hdc, title: &str, desc: &str, y: i32| {
            let card_rect = RECT {
                left: 240,
                top: y,
                right: 680,
                bottom: y + 64,
            };
            let card_pen = CreatePen(PS_SOLID, 1, card_border);
            let old_p = SelectObject(hdc, card_pen);
            let _ = SelectObject(hdc, card_brush);
            let _ = RoundRect(
                hdc,
                card_rect.left,
                card_rect.top,
                card_rect.right,
                card_rect.bottom,
                8,
                8,
            );

            // Draw Card Title
            let old_f = SelectObject(hdc, font_card_title);
            SetTextColor(hdc, text_white);
            let mut t_rect = RECT {
                left: 256,
                top: y + 10,
                right: 540,
                bottom: y + 32,
            };
            draw_text_str(
                hdc,
                title,
                &mut t_rect,
                DT_SINGLELINE | DT_VCENTER | DT_LEFT,
            );

            // Draw Card Sub-Description
            let _ = SelectObject(hdc, font_desc);
            SetTextColor(hdc, text_gray);
            let mut d_rect = RECT {
                left: 256,
                top: y + 32,
                right: 540,
                bottom: y + 54,
            };
            draw_text_str(hdc, desc, &mut d_rect, DT_SINGLELINE | DT_VCENTER | DT_LEFT);

            // Draw the elegant rounded container for our borderless input box
            // Background is matching GDI background 0x001B1214, border radius of 5px
            let container_bg = CreateSolidBrush(COLORREF(0x001B1214));
            let container_pen = CreatePen(PS_SOLID, 1, card_border); // sleek thin border
            let _ = SelectObject(hdc, container_pen);
            let _ = SelectObject(hdc, container_bg);
            let _ = RoundRect(hdc, 548, y + 17, 662, y + 47, 5, 5);
            let _ = DeleteObject(container_bg);
            let _ = DeleteObject(container_pen);

            let _ = SelectObject(hdc, old_f);
            let _ = SelectObject(hdc, old_p);
            let _ = DeleteObject(card_pen);
        };

        // 2. Draw Gorgeous Custom Toggle Card (Capsule Switch)
        let draw_toggle_card = |hdc, title: &str, desc: &str, enabled: bool, y: i32| {
            let card_rect = RECT {
                left: 240,
                top: y,
                right: 680,
                bottom: y + 64,
            };
            let card_pen = CreatePen(PS_SOLID, 1, card_border);
            let old_p = SelectObject(hdc, card_pen);
            let _ = SelectObject(hdc, card_brush);
            let _ = RoundRect(
                hdc,
                card_rect.left,
                card_rect.top,
                card_rect.right,
                card_rect.bottom,
                8,
                8,
            );

            // Draw Card Title
            let old_f = SelectObject(hdc, font_card_title);
            SetTextColor(hdc, text_white);
            let mut t_rect = RECT {
                left: 256,
                top: y + 10,
                right: 580,
                bottom: y + 32,
            };
            draw_text_str(
                hdc,
                title,
                &mut t_rect,
                DT_SINGLELINE | DT_VCENTER | DT_LEFT,
            );

            // Draw Card Sub-Description
            let _ = SelectObject(hdc, font_desc);
            SetTextColor(hdc, text_gray);
            let mut d_rect = RECT {
                left: 256,
                top: y + 32,
                right: 580,
                bottom: y + 54,
            };
            draw_text_str(hdc, desc, &mut d_rect, DT_SINGLELINE | DT_VCENTER | DT_LEFT);

            // Toggle capsule container dimensions
            let toggle_left = 610;
            let toggle_top = y + 19;
            let toggle_right = 660;
            let toggle_bottom = y + 45;

            let toggle_bg_brush = if enabled {
                CreateSolidBrush(text_accent)
            } else {
                CreateSolidBrush(COLORREF(0x002A2024))
            };
            let toggle_pen =
                CreatePen(PS_SOLID, 1, if enabled { text_accent } else { card_border });
            let _ = SelectObject(hdc, toggle_pen);
            let _ = SelectObject(hdc, toggle_bg_brush);
            let _ = RoundRect(
                hdc,
                toggle_left,
                toggle_top,
                toggle_right,
                toggle_bottom,
                13,
                13,
            );
            let _ = DeleteObject(toggle_bg_brush);
            let _ = DeleteObject(toggle_pen);

            // Sliding white circle thumb
            let thumb_radius = 8;
            let thumb_center_x = if enabled {
                toggle_right - 12
            } else {
                toggle_left + 12
            };
            let thumb_center_y = toggle_top + 13;

            let thumb_brush = CreateSolidBrush(COLORREF(0x00FFFFFF));
            let thumb_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00FFFFFF));
            let _ = SelectObject(hdc, thumb_brush);
            let _ = SelectObject(hdc, thumb_pen);
            let _ = windows::Win32::Graphics::Gdi::Ellipse(
                hdc,
                thumb_center_x - thumb_radius,
                thumb_center_y - thumb_radius,
                thumb_center_x + thumb_radius,
                thumb_center_y + thumb_radius,
            );
            let _ = DeleteObject(thumb_brush);
            let _ = DeleteObject(thumb_pen);

            let _ = SelectObject(hdc, old_f);
            let _ = SelectObject(hdc, old_p);
            let _ = DeleteObject(card_pen);
        };

        // Render respective cards for the active sidebar tab
        match state.active_tab {
            0 => {
                draw_input_card(
                    hdc,
                    "Deadzone Size",
                    "Size in pixels required to bypass normal dragging.",
                    120,
                );
                draw_input_card(
                    hdc,
                    "Snapping Threshold",
                    "Snapping distance trigger range in pixels.",
                    200,
                );
                draw_toggle_card(
                    hdc,
                    "Start with Windows",
                    "Automatically launch wingrip when you sign in.",
                    state.startup_enabled,
                    280,
                );
                draw_toggle_card(
                    hdc,
                    "Shortcut Gestures",
                    "Win+Left/Right double-click to Maximize or Minimize.",
                    state.gestures_enabled,
                    360,
                );
            }
            1 => {
                draw_input_card(
                    hdc,
                    "Tiling Gaps",
                    "Space in pixels between adjacent snapped windows.",
                    120,
                );
                draw_input_card(
                    hdc,
                    "Corner Rounded Radius",
                    "Smooth round edges for the translucent snaps.",
                    200,
                );
                draw_input_card(
                    hdc,
                    "Layout Preview Opacity",
                    "Alpha transparency of snap guide overlays.",
                    280,
                );
                draw_toggle_card(
                    hdc,
                    "Snapping Layouts",
                    "Enable translucent grid overlays and custom snapping zones.",
                    state.layouts_enabled,
                    360,
                );
                draw_toggle_card(
                    hdc,
                    "Dynamic Splitting",
                    "Suggest and split layout zones when dragging over active windows.",
                    state.split_zones_enabled,
                    440,
                );
            }
            _ => {
                // Blacklist configs
                let _ = SelectObject(hdc, font_card_title);
                SetTextColor(hdc, text_white);
                let mut blt = RECT {
                    left: 240,
                    top: 120,
                    right: 680,
                    bottom: 150,
                };
                draw_text_str(
                    hdc,
                    "Bypassed Process Executable Overrides",
                    &mut blt,
                    DT_SINGLELINE | DT_VCENTER | DT_LEFT,
                );

                // Draw the gorgeous rounded container for the blacklist multiline text box
                let container_bg = CreateSolidBrush(COLORREF(0x001B1214));
                let container_pen = CreatePen(PS_SOLID, 1, card_border);
                let old_p = SelectObject(hdc, container_pen);
                let _ = SelectObject(hdc, container_bg);
                let _ = RoundRect(hdc, 238, 158, 682, 402, 8, 8);
                let _ = SelectObject(hdc, old_p);
                let _ = DeleteObject(container_bg);
                let _ = DeleteObject(container_pen);
            }
        }

        // ----------------------------------------------------
        // DRAW SAVE ACTION BUTTON
        // ----------------------------------------------------
        let save_rect = RECT {
            left: 240,
            top: 530,
            right: 680,
            bottom: 566,
        };
        let save_pen = CreatePen(
            PS_SOLID,
            1,
            if state.hovered == HoveredControl::SaveButton {
                text_accent
            } else {
                card_border
            },
        );
        let old_pen_save = SelectObject(hdc, save_pen);

        if state.hovered == HoveredControl::SaveButton {
            let hover_accent = CreateSolidBrush(COLORREF(0x007842B8));
            let _ = SelectObject(hdc, hover_accent);
            let _ = RoundRect(
                hdc,
                save_rect.left,
                save_rect.top,
                save_rect.right,
                save_rect.bottom,
                8,
                8,
            );
            let _ = DeleteObject(hover_accent);
        } else {
            let _ = SelectObject(hdc, card_brush);
            let _ = RoundRect(
                hdc,
                save_rect.left,
                save_rect.top,
                save_rect.right,
                save_rect.bottom,
                8,
                8,
            );
        }

        let old_f = SelectObject(hdc, font_card_title);
        SetTextColor(hdc, text_white);
        let mut save_t = save_rect;
        draw_text_str(
            hdc,
            "Save & Apply Settings",
            &mut save_t,
            DT_SINGLELINE | DT_VCENTER | DT_CENTER,
        );

        // cleanup typography and graphics resources
        let _ = SelectObject(hdc, old_f);
        let _ = SelectObject(hdc, old_f_t);
        let _ = SelectObject(hdc, old_pen_save);
        let _ = DeleteObject(border_pen);
        let _ = DeleteObject(save_pen);
        let _ = DeleteObject(font_title);
        let _ = DeleteObject(font_card_title);
        let _ = DeleteObject(font_body);
        let _ = DeleteObject(font_desc);
        let _ = DeleteObject(bg_brush);
        let _ = DeleteObject(sidebar_brush);
        let _ = DeleteObject(card_brush);
        let _ = DeleteObject(hover_brush);
    }
}
