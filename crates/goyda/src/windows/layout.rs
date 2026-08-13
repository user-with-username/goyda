use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{MoveWindow, SetWindowPos, SWP_NOMOVE, SWP_NOZORDER};

use crate::components::{Align, LayoutDirection};

use super::state::{self, ControlKind};

/// Resizes `hwnd` to `width`x`height` (keeping its current position - sizing
/// is all a resize ever needs to change for the control being resized
/// itself) and, if it's a stack panel, re-flows its children to fit,
/// honoring the panel's `AlignItems` (cross-axis)/`JustifyContent`
/// (main-axis) settings - each child stretches or packs to its natural size
/// along the cross axis depending on `AlignItems`, and the whole run of
/// children is positioned along the main axis depending on
/// `JustifyContent`, recursing into any child that's itself a panel. Called
/// once for the whole mounted tree on every `WM_SIZE` (see `windows/mod.rs`)
/// - this is what makes the window resizable instead of leaving the content
/// pinned at whatever size it first measured to.
pub fn relayout(hwnd: HWND, width: i32, height: i32) {
    unsafe {
        SetWindowPos(hwnd, std::ptr::null_mut(), 0, 0, width.max(0), height.max(0), SWP_NOMOVE | SWP_NOZORDER);
    }

    let panel = state::with_state(hwnd, |s| {
        let scrollable = matches!(s.kind, ControlKind::ScrollView { .. });
        match &s.kind {
            ControlKind::Panel { direction, spacing, children } | ControlKind::ScrollView { direction, spacing, children } => Some((
                *direction,
                *spacing,
                children.clone(),
                s.padding.clone(),
                s.align_items,
                s.justify_content,
                scrollable,
                s.scroll_offset,
            )),
            _ => None,
        }
    })
    .flatten();

    let Some((direction, spacing, children, padding, align_items, justify_content, scrollable, scroll_offset)) = panel else { return };

    let content_x = padding.left;
    let content_y = padding.top;
    let content_w = (width - padding.left - padding.right).max(0);
    let content_h = (height - padding.top - padding.bottom).max(0);
    let content_main = match direction {
        LayoutDirection::Horizontal => content_w,
        LayoutDirection::Vertical => content_h,
    };

    // Images opt out of cross-axis stretch and keep their own aspect ratio
    // (matching the web backend's `align-self: flex-start` on `<img>`) -
    // stretching a bitmap to fill the cross axis distorts it, unlike
    // text/buttons/panels which just get more background. This opt-out is
    // independent of `AlignItems`: an image never stretches, but still
    // participates in whatever cross-axis position `AlignItems` picks.
    let sizes: Vec<((i32, i32), bool)> = children
        .iter()
        .map(|c| {
            let is_image = state::with_state(*c, |s| matches!(s.kind, ControlKind::Image { .. })).unwrap_or(false);
            (state::natural_size(*c), is_image)
        })
        .collect();

    let main_sizes: Vec<i32> = sizes
        .iter()
        .map(|((w, h), _)| match direction {
            LayoutDirection::Horizontal => *w,
            LayoutDirection::Vertical => *h,
        })
        .collect();

    let n = children.len();
    let total_natural_main: i32 = main_sizes.iter().sum::<i32>() + spacing * (n.saturating_sub(1)) as i32;
    let slack = (content_main - total_natural_main).max(0);

    // `JustifyContent` only has slack to work with once every child already
    // has its natural main-axis size - `Start` (the default) matches the
    // packed-from-the-top/left behavior this always had.
    let (mut cursor, gap) = match justify_content {
        Align::Start | Align::Stretch => (0, spacing),
        Align::Center => (slack / 2, spacing),
        Align::End => (slack, spacing),
        Align::SpaceBetween if n > 1 => (0, spacing + slack / (n as i32 - 1)),
        Align::SpaceBetween => (slack / 2, spacing),
    };

    for (i, child) in children.iter().enumerate() {
        let ((natural_w, natural_h), is_image) = sizes[i];
        let main = main_sizes[i];
        let cross_natural = match direction {
            LayoutDirection::Horizontal => natural_h,
            LayoutDirection::Vertical => natural_w,
        };
        let content_cross = match direction {
            LayoutDirection::Horizontal => content_h,
            LayoutDirection::Vertical => content_w,
        };

        let (cross_size, cross_offset) = if is_image {
            (cross_natural, 0)
        } else {
            match align_items {
                Align::Stretch => (content_cross, 0),
                Align::Start => (cross_natural, 0),
                Align::Center => (cross_natural, (content_cross - cross_natural).max(0) / 2),
                Align::End => (cross_natural, (content_cross - cross_natural).max(0)),
                Align::SpaceBetween => (content_cross, 0),
            }
        };

        let scrolled_cursor = if scrollable { cursor - scroll_offset } else { cursor };
        let (x, y, cw, ch) = match direction {
            LayoutDirection::Horizontal => (content_x + scrolled_cursor, content_y + cross_offset, main, cross_size),
            LayoutDirection::Vertical => (content_x + cross_offset, content_y + scrolled_cursor, cross_size, main),
        };

        unsafe {
            MoveWindow(*child, x, y, cw.max(0), ch.max(0), 1);
        }
        relayout(*child, cw, ch);

        cursor += main + gap;
    }
}

/// The total main-axis extent `hwnd`'s children would need if laid out at
/// their natural size (ignoring any current scroll offset) - what
/// [`crate::windows::scroll_by`] clamps a `ScrollView`'s scroll position
/// against. `None` if `hwnd` isn't a panel/scroll view.
pub fn content_main_size(hwnd: HWND) -> Option<(LayoutDirection, i32)> {
    let panel = state::with_state(hwnd, |s| match &s.kind {
        ControlKind::Panel { direction, spacing, children } | ControlKind::ScrollView { direction, spacing, children } => {
            Some((*direction, *spacing, children.clone()))
        }
        _ => None,
    })
    .flatten()?;

    let (direction, spacing, children) = panel;
    let main_sizes: Vec<i32> = children
        .iter()
        .map(|c| {
            let (w, h) = state::natural_size(*c);
            match direction {
                LayoutDirection::Horizontal => w,
                LayoutDirection::Vertical => h,
            }
        })
        .collect();

    let n = children.len();
    let total = main_sizes.iter().sum::<i32>() + spacing * n.saturating_sub(1) as i32;
    Some((direction, total))
}
