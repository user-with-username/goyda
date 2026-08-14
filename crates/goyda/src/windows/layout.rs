use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{MoveWindow, SetWindowPos, SWP_NOMOVE, SWP_NOZORDER};

use crate::components::{Align, LayoutDirection};

use super::state::{self, ControlKind};

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

    let mut main_sizes: Vec<i32> = sizes
        .iter()
        .map(|((w, h), _)| match direction {
            LayoutDirection::Horizontal => *w,
            LayoutDirection::Vertical => *h,
        })
        .collect();

    // A scrollable panel with exactly one child (e.g. the whole-page wrap
    // `windows::mod::mount` builds so a page taller than the window can
    // scroll - see its own doc comment) needs that child to still fill the
    // viewport when it's the *shorter* one, the same "grow to fill, but
    // only shrink to natural size, never below the viewport" Android gets
    // for free from `ScrollView.setFillViewport(true)` (see
    // `android::bootstrap::wrap_scrollable`) - without this, a page
    // shorter than the window only paints its own background over its own
    // short height, leaving a plain unstyled band underneath. A multi-child
    // scrollable list (e.g. `styles_page`'s scrollable row list) is left
    // alone: stretching every item to fill the viewport would break the
    // list, not just the empty-space case this is actually fixing.
    if scrollable && main_sizes.len() == 1 {
        main_sizes[0] = main_sizes[0].max(content_main);
    }

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
