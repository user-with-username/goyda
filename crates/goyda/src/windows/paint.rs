use windows_sys::Win32::Foundation::{COLORREF, HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::GetParent;

use crate::components::Align;

use super::state::{client_rect, with_state, ControlKind};

fn text_align_flag(align: Align) -> u32 {
    match align {
        Align::Start | Align::Stretch | Align::SpaceBetween => DT_LEFT,
        Align::Center => DT_CENTER,
        Align::End => DT_RIGHT,
    }
}

fn ancestor_background(hwnd: HWND) -> COLORREF {
    let mut current = unsafe { GetParent(hwnd) };
    while !current.is_null() {
        if let Some(bg) = with_state(current, |s| s.background_color).flatten() {
            return bg;
        }
        current = unsafe { GetParent(current) };
    }
    0x00FF_FFFF
}

pub fn paint_control(hwnd: HWND, hdc: HDC) {
    let rect = client_rect(hwnd);

    // `WM_ERASEBKGND` is suppressed entirely (see `state::wndproc`) to avoid
    // flicker, which means this is the *only* place a control's background
    // ever gets cleared - without it, redrawing new content (e.g. a
    // `Signal`-driven text update) would draw right on top of whatever was
    // there before instead of replacing it.
    let base = with_state(hwnd, |s| s.background_color).flatten().unwrap_or_else(|| ancestor_background(hwnd));
    unsafe {
        let base_brush = CreateSolidBrush(base);
        FillRect(hdc, &rect, base_brush);
        DeleteObject(base_brush);
    }

    with_state(hwnd, |state| unsafe {
        // Shadow: a soft approximation - an offset, slightly larger filled
        // rect drawn first, in the same border color if set (else a plain
        // gray), so it peeks out from behind the control.
        if state.shadow > 0 {
            let color = state.shadow_color.or(state.border_color).unwrap_or(0x00C8_C8C8);
            let brush = CreateSolidBrush(color);
            let shadow_rect =
                RECT { left: rect.left + state.shadow, top: rect.top + state.shadow, right: rect.right + state.shadow, bottom: rect.bottom + state.shadow };
            FillRect(hdc, &shadow_rect, brush);
            DeleteObject(brush);
        }

        if let Some(bg) = state.background_color {
            let brush = CreateSolidBrush(bg);
            if state.border_radius > 0 {
                let pen = CreatePen(PS_NULL as i32, 0, 0);
                let old_pen = SelectObject(hdc, pen);
                let old_brush = SelectObject(hdc, brush);
                RoundRect(hdc, rect.left, rect.top, rect.right, rect.bottom, state.border_radius * 2, state.border_radius * 2);
                SelectObject(hdc, old_pen);
                SelectObject(hdc, old_brush);
                DeleteObject(pen);
            } else {
                FillRect(hdc, &rect, brush);
            }
            DeleteObject(brush);
        }

        if state.border_width > 0 {
            if let Some(border) = state.border_color {
                let pen = CreatePen(PS_SOLID as i32, state.border_width, border);
                let old_pen = SelectObject(hdc, pen);
                let old_brush = SelectObject(hdc, GetStockObject(NULL_BRUSH));
                if state.border_radius > 0 {
                    RoundRect(hdc, rect.left, rect.top, rect.right, rect.bottom, state.border_radius * 2, state.border_radius * 2);
                } else {
                    Rectangle(hdc, rect.left, rect.top, rect.right, rect.bottom);
                }
                SelectObject(hdc, old_pen);
                SelectObject(hdc, old_brush);
                DeleteObject(pen);
            }
        }

        let content_rect = RECT {
            left: rect.left + state.padding.left,
            top: rect.top + state.padding.top,
            right: rect.right - state.padding.right,
            bottom: rect.bottom - state.padding.bottom,
        };

        match &state.kind {
            ControlKind::Text(text) | ControlKind::Button(text) => {
                if let Some(font) = state.font {
                    SelectObject(hdc, font);
                }
                SetBkMode(hdc, TRANSPARENT as i32);
                SetTextCharacterExtra(hdc, state.letter_spacing);
                SetTextColor(hdc, state.text_color.unwrap_or(0x0000_0000));
                let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
                let mut r = content_rect;
                let ellipsis_flag = if state.ellipsis { DT_END_ELLIPSIS } else { 0 };
                DrawTextW(hdc, wide.as_ptr(), -1, &mut r, text_align_flag(state.text_align) | DT_VCENTER | DT_SINGLELINE | ellipsis_flag);
            }
            ControlKind::TextInput { text, placeholder, multiline } => {
                if let Some(font) = state.font {
                    SelectObject(hdc, font);
                }
                SetBkMode(hdc, TRANSPARENT as i32);
                SetTextCharacterExtra(hdc, state.letter_spacing);
                let (shown, color) = if text.is_empty() {
                    (placeholder.as_str(), 0x00A0_A0A0)
                } else {
                    (text.as_str(), state.text_color.unwrap_or(0x0000_0000))
                };
                SetTextColor(hdc, color);
                let mut r = content_rect;
                if *multiline {
                    if let Some(lh) = state.line_height {
                        // Custom line spacing needs each line drawn (and
                        // stepped down `lh` px) by hand - GDI's own
                        // `DT_WORDBREAK` layout engine has no line-height
                        // parameter of its own. Trade-off: lines only break
                        // on real `\n`s in this mode (no auto word-wrap),
                        // since we don't control where the engine would
                        // have wrapped otherwise.
                        let mut y = r.top;
                        for line in shown.split('\n') {
                            let wide_line: Vec<u16> = line.encode_utf16().chain(std::iter::once(0)).collect();
                            let mut line_rect = RECT { left: r.left, top: y, right: r.right, bottom: y + lh };
                            DrawTextW(hdc, wide_line.as_ptr(), -1, &mut line_rect, text_align_flag(state.text_align) | DT_TOP | DT_SINGLELINE | DT_NOCLIP);
                            y += lh;
                        }
                    } else {
                        let wide: Vec<u16> = shown.encode_utf16().chain(std::iter::once(0)).collect();
                        DrawTextW(hdc, wide.as_ptr(), -1, &mut r, text_align_flag(state.text_align) | DT_TOP | DT_WORDBREAK);
                    }
                } else {
                    let wide: Vec<u16> = shown.encode_utf16().chain(std::iter::once(0)).collect();
                    let ellipsis_flag = if state.ellipsis { DT_END_ELLIPSIS } else { 0 };
                    DrawTextW(hdc, wide.as_ptr(), -1, &mut r, text_align_flag(state.text_align) | DT_VCENTER | DT_SINGLELINE | ellipsis_flag);
                }
            }
            ControlKind::Checkbox { label } => {
                let box_size = 16;
                let box_top = rect.top + (rect.bottom - rect.top - box_size) / 2;
                let box_rect = RECT { left: rect.left, top: box_top, right: rect.left + box_size, bottom: box_top + box_size };

                let border_pen = CreatePen(PS_SOLID as i32, 1, 0x0090_9090);
                let old_pen = SelectObject(hdc, border_pen);
                let fill_brush = CreateSolidBrush(if state.checked { 0x00D0_7800 } else { 0x00FF_FFFF });
                let old_brush = SelectObject(hdc, fill_brush);
                Rectangle(hdc, box_rect.left, box_rect.top, box_rect.right, box_rect.bottom);
                SelectObject(hdc, old_pen);
                SelectObject(hdc, old_brush);
                DeleteObject(border_pen);
                DeleteObject(fill_brush);

                if state.checked {
                    let check_pen = CreatePen(PS_SOLID as i32, 2, 0x00FF_FFFF);
                    let old = SelectObject(hdc, check_pen);
                    MoveToEx(hdc, box_rect.left + 3, box_rect.top + 8, std::ptr::null_mut());
                    LineTo(hdc, box_rect.left + 6, box_rect.top + 11);
                    LineTo(hdc, box_rect.left + 13, box_rect.top + 4);
                    SelectObject(hdc, old);
                    DeleteObject(check_pen);
                }

                if !label.is_empty() {
                    if let Some(font) = state.font {
                        SelectObject(hdc, font);
                    }
                    SetBkMode(hdc, TRANSPARENT as i32);
                    SetTextColor(hdc, state.text_color.unwrap_or(0x0000_0000));
                    let wide: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
                    let mut r = RECT { left: box_rect.right + 8, top: rect.top, right: rect.right, bottom: rect.bottom };
                    DrawTextW(hdc, wide.as_ptr(), -1, &mut r, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
                }
            }
            ControlKind::RadioButton { label, .. } => {
                let box_size = 16;
                let box_top = rect.top + (rect.bottom - rect.top - box_size) / 2;
                let box_rect = RECT { left: rect.left, top: box_top, right: rect.left + box_size, bottom: box_top + box_size };

                let border_pen = CreatePen(PS_SOLID as i32, 1, 0x0090_9090);
                let old_pen = SelectObject(hdc, border_pen);
                let fill_brush = CreateSolidBrush(0x00FF_FFFF);
                let old_brush = SelectObject(hdc, fill_brush);
                Ellipse(hdc, box_rect.left, box_rect.top, box_rect.right, box_rect.bottom);
                SelectObject(hdc, old_pen);
                SelectObject(hdc, old_brush);
                DeleteObject(border_pen);
                DeleteObject(fill_brush);

                if state.checked {
                    let inset = 4;
                    let dot_brush = CreateSolidBrush(0x00D0_7800);
                    let pen = CreatePen(PS_NULL as i32, 0, 0);
                    let old_pen = SelectObject(hdc, pen);
                    let old_brush = SelectObject(hdc, dot_brush);
                    Ellipse(hdc, box_rect.left + inset, box_rect.top + inset, box_rect.right - inset, box_rect.bottom - inset);
                    SelectObject(hdc, old_pen);
                    SelectObject(hdc, old_brush);
                    DeleteObject(dot_brush);
                    DeleteObject(pen);
                }

                if !label.is_empty() {
                    if let Some(font) = state.font {
                        SelectObject(hdc, font);
                    }
                    SetBkMode(hdc, TRANSPARENT as i32);
                    SetTextColor(hdc, state.text_color.unwrap_or(0x0000_0000));
                    let wide: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
                    let mut r = RECT { left: box_rect.right + 8, top: rect.top, right: rect.right, bottom: rect.bottom };
                    DrawTextW(hdc, wide.as_ptr(), -1, &mut r, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
                }
            }
            ControlKind::Switch => {
                let (track_w, track_h) = (40, 22);
                let track_top = rect.top + (rect.bottom - rect.top - track_h) / 2;
                let track_rect = RECT { left: rect.left, top: track_top, right: rect.left + track_w, bottom: track_top + track_h };

                let track_brush = CreateSolidBrush(if state.checked { 0x00D0_7800 } else { 0x00C8_C8C8 });
                let old_brush = SelectObject(hdc, track_brush);
                let pen = CreatePen(PS_NULL as i32, 0, 0);
                let old_pen = SelectObject(hdc, pen);
                RoundRect(hdc, track_rect.left, track_rect.top, track_rect.right, track_rect.bottom, track_h, track_h);
                SelectObject(hdc, old_pen);
                SelectObject(hdc, old_brush);
                DeleteObject(track_brush);
                DeleteObject(pen);

                let thumb_d = track_h - 4;
                let thumb_left = if state.checked { track_rect.right - thumb_d - 2 } else { track_rect.left + 2 };
                let thumb_top = track_rect.top + 2;
                let thumb_brush = CreateSolidBrush(0x00FF_FFFF);
                let old_brush = SelectObject(hdc, thumb_brush);
                Ellipse(hdc, thumb_left, thumb_top, thumb_left + thumb_d, thumb_top + thumb_d);
                SelectObject(hdc, old_brush);
                DeleteObject(thumb_brush);
            }
            ControlKind::Progress { value } => {
                let filled_w = ((rect.right - rect.left) as f32 * value.clamp(0.0, 1.0)) as i32;
                if filled_w > 0 {
                    let fill_rect = RECT { left: rect.left, top: rect.top, right: rect.left + filled_w, bottom: rect.bottom };
                    let brush = CreateSolidBrush(0x00D0_7800);
                    let pen = CreatePen(PS_NULL as i32, 0, 0);
                    let old_pen = SelectObject(hdc, pen);
                    let old_brush = SelectObject(hdc, brush);
                    let radius = state.border_radius * 2;
                    RoundRect(hdc, fill_rect.left, fill_rect.top, fill_rect.right, fill_rect.bottom, radius, radius);
                    SelectObject(hdc, old_pen);
                    SelectObject(hdc, old_brush);
                    DeleteObject(brush);
                    DeleteObject(pen);
                }
            }
            ControlKind::Image { bitmap: Some(bmp), width, height } => {
                let mem_dc = CreateCompatibleDC(hdc);
                let old = SelectObject(mem_dc, *bmp as _);

                let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
                let ok = AlphaBlend(
                    hdc,
                    content_rect.left,
                    content_rect.top,
                    content_rect.right - content_rect.left,
                    content_rect.bottom - content_rect.top,
                    mem_dc,
                    0,
                    0,
                    *width,
                    *height,
                    blend,
                );
                if ok == 0 {
                    StretchBlt(
                        hdc,
                        content_rect.left,
                        content_rect.top,
                        content_rect.right - content_rect.left,
                        content_rect.bottom - content_rect.top,
                        mem_dc,
                        0,
                        0,
                        *width,
                        *height,
                        SRCCOPY,
                    );
                }

                SelectObject(mem_dc, old);
                DeleteDC(mem_dc);
            }
            ControlKind::Image { bitmap: None, .. }
            | ControlKind::Panel { .. }
            | ControlKind::ScrollView { .. }
            | ControlKind::Overlay { .. }
            | ControlKind::Spacer
            | ControlKind::Divider => {}
        }
    });
}
