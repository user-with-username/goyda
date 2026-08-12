use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{MoveWindow, SetWindowPos, SWP_NOMOVE, SWP_NOZORDER};

use crate::components::LayoutDirection;

use super::state::{self, ControlKind};

/// Resizes `hwnd` to `width`x`height` (keeping its current position - sizing
/// is all a resize ever needs to change for the control being resized
/// itself) and, if it's a stack panel, re-flows its children to fit: each
/// child stretches to fill the panel's cross axis and keeps its own natural
/// size along the main axis, recursing into any child that's itself a
/// panel. Called once for the whole mounted tree on every `WM_SIZE` (see
/// `windows/mod.rs`) - this is what makes the window resizable instead of
/// leaving the content pinned at whatever size it first measured to.
pub fn relayout(hwnd: HWND, width: i32, height: i32) {
    unsafe {
        SetWindowPos(hwnd, std::ptr::null_mut(), 0, 0, width.max(0), height.max(0), SWP_NOMOVE | SWP_NOZORDER);
    }

    let panel = state::with_state(hwnd, |s| match &s.kind {
        ControlKind::Panel { direction, spacing, children } => Some((*direction, *spacing, children.clone(), s.padding.clone())),
        _ => None,
    })
    .flatten();

    let Some((direction, spacing, children, padding)) = panel else { return };

    let content_x = padding.left;
    let content_y = padding.top;
    let content_w = (width - padding.left - padding.right).max(0);
    let content_h = (height - padding.top - padding.bottom).max(0);

    let mut cursor = match direction {
        LayoutDirection::Horizontal => content_x,
        LayoutDirection::Vertical => content_y,
    };

    for child in &children {
        let (natural_w, natural_h) = state::natural_size(*child);

        // Images opt out of cross-axis stretch and keep their own aspect
        // ratio (matching the web backend's `align-self: flex-start` on
        // `<img>`) - stretching a bitmap to fill the cross axis distorts it,
        // unlike text/buttons/panels which just get more background.
        let is_image = state::with_state(*child, |s| matches!(s.kind, ControlKind::Image { .. })).unwrap_or(false);

        let (x, y, cw, ch) = match (direction, is_image) {
            (LayoutDirection::Horizontal, false) => (cursor, content_y, natural_w, content_h),
            (LayoutDirection::Horizontal, true) => (cursor, content_y, natural_w, natural_h),
            (LayoutDirection::Vertical, false) => (content_x, cursor, content_w, natural_h),
            (LayoutDirection::Vertical, true) => (content_x, cursor, natural_w, natural_h),
        };

        unsafe {
            MoveWindow(*child, x, y, cw.max(0), ch.max(0), 1);
        }
        relayout(*child, cw, ch);

        let main = match direction {
            LayoutDirection::Horizontal => natural_w,
            LayoutDirection::Vertical => natural_h,
        };
        cursor += main + spacing;
    }
}
