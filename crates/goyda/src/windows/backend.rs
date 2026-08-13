use windows_sys::Win32::Foundation::{HINSTANCE, HWND, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{GetDC, GetTextExtentPoint32W, InvalidateRect, ReleaseDC, SelectObject, HFONT};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, SetFocus};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::components::{Asset, Axis, Edge, LayoutDirection, StyleProperty, StyleValue};
use crate::core::events::Update;
use crate::core::{Backend, BackendUpdater};

use super::font;
use super::image::decode_to_bitmap;
use super::state::{self, ControlKind, ControlState, Edges};

const DEFAULT_FONT_SIZE: i32 = 16;
/// A default inset for buttons, matching the android backend's own
/// clickable-default-padding constant - without it a button sized to its
/// bare text extent would be uncomfortably small to click.
const BUTTON_PADDING: Edges = Edges { left: 16, top: 10, right: 16, bottom: 10 };

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn measure_text(text: &str, font: HFONT) -> (i32, i32) {
    unsafe {
        let hdc = GetDC(std::ptr::null_mut());
        let old = SelectObject(hdc, font as _);
        let w = wide(text);
        let mut size = windows_sys::Win32::Foundation::SIZE::default();
        GetTextExtentPoint32W(hdc, w.as_ptr(), (w.len() - 1) as i32, &mut size);
        SelectObject(hdc, old);
        ReleaseDC(std::ptr::null_mut(), hdc);
        (size.cx, size.cy)
    }
}

#[derive(Clone, Copy)]
pub struct WindowsView {
    pub hwnd: HWND,
}

#[derive(Clone)]
pub struct WindowsUpdater;

impl BackendUpdater for WindowsUpdater {
    type PlatformView = WindowsView;

    fn apply_update(&mut self, view: &Self::PlatformView, update: Update) {
        match update {
            Update::SetText(content) => {
                state::with_state(view.hwnd, |s| match &mut s.kind {
                    ControlKind::Text(t) | ControlKind::Button(t) => *t = content,
                    ControlKind::TextInput { text, .. } => *text = content,
                    _ => {}
                });
                let (w, h) = measure_text_for(view.hwnd);
                state::with_state(view.hwnd, |s| s.natural_size = (w, h));
                invalidate(view.hwnd);
            }
            Update::SetProgress(value) => {
                state::with_state(view.hwnd, |s| {
                    if let ControlKind::Progress { value: v } = &mut s.kind {
                        *v = value.clamp(0.0, 1.0);
                    }
                });
                invalidate(view.hwnd);
            }
        }
    }
}

/// Minimum width for an empty [`ControlKind::TextInput`] - without it, a
/// freshly-created field with no placeholder/text would measure to zero
/// width and be invisible.
const TEXT_INPUT_MIN_WIDTH: i32 = 150;
const CHECKBOX_BOX_SIZE: i32 = 16;
const SWITCH_SIZE: (i32, i32) = (40, 22);
const PROGRESS_HEIGHT: i32 = 8;
const PROGRESS_DEFAULT_WIDTH: i32 = 200;
const DIVIDER_THICKNESS: i32 = 1;

fn measure_text_for(hwnd: HWND) -> (i32, i32) {
    state::with_state(hwnd, |s| {
        let is_button = matches!(s.kind, ControlKind::Button(_));
        let is_input = matches!(s.kind, ControlKind::TextInput { .. });
        let text = match &s.kind {
            ControlKind::Text(t) | ControlKind::Button(t) => t.clone(),
            ControlKind::TextInput { text, placeholder } => {
                if text.is_empty() { placeholder.clone() } else { text.clone() }
            }
            _ => return (0, 0),
        };
        let f = s.font.unwrap_or_else(|| font::default_font(DEFAULT_FONT_SIZE));
        let (w, h) = measure_text(&text, f);
        let pad = if is_button { &BUTTON_PADDING } else { &s.padding };
        let total_w = w + pad.left + pad.right;
        let total_h = h + pad.top + pad.bottom;
        if is_input {
            (total_w.max(TEXT_INPUT_MIN_WIDTH), total_h.max(24))
        } else {
            (total_w, total_h)
        }
    })
    .unwrap_or((0, 0))
}

fn invalidate(hwnd: HWND) {
    unsafe {
        InvalidateRect(hwnd, std::ptr::null(), 1);
    }
}

/// Built-in click-to-toggle for [`ControlKind::Checkbox`]/[`ControlKind::Switch`],
/// matching what a real `<input type=checkbox>`/Android `CheckBox`/`Switch`
/// already does for free on the other two backends - independent of whether
/// the app attaches `.on_checked_change(...)`, which only needs to *observe*
/// this (see `crate::listeners`'s `checked_change` arm and
/// [`crate::windows::is_self_toggling`]).
fn register_click_to_toggle(hwnd: HWND) {
    state::register_raw_hook(hwnd, std::rc::Rc::new(move |msg, _wparam, _lparam| {
        if msg == WM_LBUTTONUP {
            crate::windows::toggle_checked(hwnd);
        }
    }));
}

pub struct WindowsBackend {
    pub hinstance: HINSTANCE,
    pub parent: HWND,
}

impl WindowsBackend {
    pub fn new(hinstance: HINSTANCE, parent: HWND) -> Self {
        Self { hinstance, parent }
    }

    fn create_control(&mut self, kind: ControlKind) -> WindowsView {
        let class = wide(super::state::CLASS_NAME.trim_end_matches('\0'));
        // No `WS_EX_LAYERED` here: it's only added lazily (see `Axis::Opacity`
        // in `apply_style`) when a control actually needs it. Layered *child*
        // windows have historically been unreliable about picking up normal
        // `WM_PAINT` output - setting it unconditionally on every control
        // left everything rendering blank.
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE,
                0,
                0,
                0,
                0,
                self.parent,
                std::ptr::null_mut(),
                self.hinstance,
                std::ptr::null(),
            )
        };
        state::insert_state(hwnd, ControlState::new(kind));
        WindowsView { hwnd }
    }
}

impl Backend for WindowsBackend {
    type PlatformView = WindowsView;
    type Updater = WindowsUpdater;

    fn clone_updater(&self) -> Self::Updater {
        WindowsUpdater
    }

    fn create_text(&mut self, content: &str) -> Self::PlatformView {
        let view = self.create_control(ControlKind::Text(content.to_string()));
        let (w, h) = measure_text_for(view.hwnd);
        state::with_state(view.hwnd, |s| s.natural_size = (w, h));
        view
    }

    fn create_button(&mut self, text: &str) -> Self::PlatformView {
        let view = self.create_control(ControlKind::Button(text.to_string()));
        let (w, h) = measure_text_for(view.hwnd);
        state::with_state(view.hwnd, |s| {
            s.natural_size = (w, h);
            s.background_color = Some(state::to_colorref(0xFFE0E0E0));
            s.border_radius = 4;
        });
        view
    }

    fn create_image(&mut self, asset: &Asset) -> Self::PlatformView {
        let (kind, size) = match decode_to_bitmap(asset) {
            Some((bmp, w, h)) => (ControlKind::Image { bitmap: Some(bmp), width: w, height: h }, (w, h)),
            None => (ControlKind::Image { bitmap: None, width: 0, height: 0 }, (0, 0)),
        };
        let view = self.create_control(kind);
        state::with_state(view.hwnd, |s| s.natural_size = size);
        view
    }

    fn create_text_input(&mut self, placeholder: &str, initial_text: &str) -> Self::PlatformView {
        let view = self.create_control(ControlKind::TextInput { text: initial_text.to_string(), placeholder: placeholder.to_string() });
        state::with_state(view.hwnd, |s| {
            s.text_buffer = initial_text.to_string();
            s.background_color = Some(state::to_colorref(0xFFFFFFFF));
            s.border_color = Some(state::to_colorref(0xFF969696));
            s.border_width = 1;
            s.border_radius = 2;
            s.padding = Edges { left: 8, top: 4, right: 8, bottom: 4 };
        });
        let (w, h) = measure_text_for(view.hwnd);
        state::with_state(view.hwnd, |s| s.natural_size = (w, h));

        // Built-in interactivity, matching what a real `<input>`/`EditText`
        // gives for free on the other two backends: click to focus (nothing
        // else on this backend ever calls `SetFocus`, so without this a
        // freshly-created field could never receive `WM_CHAR` at all), then
        // accumulate typed characters (and handle backspace) straight into
        // this control's own buffer - independent of whether the app
        // attaches `.on_text_changed(...)`, which only needs to *observe*
        // this, not drive it (see `crate::listeners`'s `text_watcher` arm).
        let hwnd = view.hwnd;
        state::register_raw_hook(hwnd, std::rc::Rc::new(move |msg, wparam, _lparam| unsafe {
            match msg {
                WM_LBUTTONDOWN => {
                    SetFocus(hwnd);
                }
                WM_CHAR => {
                    if wparam == 0x08 {
                        crate::windows::backspace_text_buffer(hwnd);
                    } else if let Some(ch) = char::from_u32(wparam as u32) {
                        if !ch.is_control() {
                            crate::windows::append_text_buffer(hwnd, ch);
                        }
                    }
                }
                _ => {}
            }
        }));

        view
    }

    fn create_checkbox(&mut self, label: &str, checked: bool) -> Self::PlatformView {
        let view = self.create_control(ControlKind::Checkbox { label: label.to_string() });
        let font = font::default_font(DEFAULT_FONT_SIZE);
        let (label_w, label_h) = if label.is_empty() { (0, 0) } else { measure_text(label, font) };
        let gap = if label.is_empty() { 0 } else { 8 };
        let w = CHECKBOX_BOX_SIZE + gap + label_w;
        let h = CHECKBOX_BOX_SIZE.max(label_h);
        state::with_state(view.hwnd, |s| {
            s.checked = checked;
            s.natural_size = (w, h);
        });
        register_click_to_toggle(view.hwnd);
        view
    }

    fn create_switch(&mut self, checked: bool) -> Self::PlatformView {
        let view = self.create_control(ControlKind::Switch);
        state::with_state(view.hwnd, |s| {
            s.checked = checked;
            s.natural_size = SWITCH_SIZE;
        });
        register_click_to_toggle(view.hwnd);
        view
    }

    fn create_progress(&mut self, value: f32) -> Self::PlatformView {
        let view = self.create_control(ControlKind::Progress { value: value.clamp(0.0, 1.0) });
        state::with_state(view.hwnd, |s| {
            s.natural_size = (PROGRESS_DEFAULT_WIDTH, PROGRESS_HEIGHT);
            s.border_radius = PROGRESS_HEIGHT / 2;
            s.background_color = Some(state::to_colorref(0xFFE0E0E0));
        });

        // A scrubber, not just a read-only indicator: clicking or dragging
        // sets its own value immediately (matching a native `<input
        // type=range>`/Android `SeekBar`, which both do the same for free) -
        // independent of whether the app attaches `.on_value_changed(...)`,
        // which only needs to *observe* this (see `crate::listeners`'s
        // `seek` arm). `WM_MOUSEMOVE` only counts as a drag while the left
        // button is actually held (`MK_LBUTTON`).
        const MK_LBUTTON: WPARAM = 0x0001;
        let hwnd = view.hwnd;
        state::register_raw_hook(hwnd, std::rc::Rc::new(move |msg, wparam, lparam| unsafe {
            match msg {
                WM_LBUTTONDOWN => {
                    SetCapture(hwnd);
                    crate::windows::set_progress_from_x(hwnd, (lparam & 0xFFFF) as i16 as i32);
                }
                WM_MOUSEMOVE if wparam & MK_LBUTTON != 0 => {
                    crate::windows::set_progress_from_x(hwnd, (lparam & 0xFFFF) as i16 as i32);
                }
                WM_LBUTTONUP => {
                    ReleaseCapture();
                }
                _ => {}
            }
        }));

        view
    }

    fn create_spacer(&mut self, size: i32) -> Self::PlatformView {
        let view = self.create_control(ControlKind::Spacer);
        state::with_state(view.hwnd, |s| s.natural_size = (size, size));
        view
    }

    fn create_divider(&mut self) -> Self::PlatformView {
        let view = self.create_control(ControlKind::Divider);
        state::with_state(view.hwnd, |s| {
            s.natural_size = (DIVIDER_THICKNESS, DIVIDER_THICKNESS);
            s.background_color = Some(state::to_colorref(0xFFC8C8C8));
        });
        view
    }

    fn create_stack(&mut self, direction: LayoutDirection, spacing: i32, children: Vec<Self::PlatformView>) -> Self::PlatformView {
        let panel = self.create_control(ControlKind::Panel {
            direction,
            spacing,
            children: children.iter().map(|c| c.hwnd).collect(),
        });

        let sizes: Vec<(i32, i32)> = children.iter().map(|c| state::natural_size(c.hwnd)).collect();
        let mut cursor = 0i32;
        let mut cross_max = 0i32;

        for (child, (w, h)) in children.iter().zip(sizes.iter()) {
            unsafe {
                SetParent(child.hwnd, panel.hwnd);
                let (x, y) = match direction {
                    LayoutDirection::Horizontal => (cursor, 0),
                    LayoutDirection::Vertical => (0, cursor),
                };
                MoveWindow(child.hwnd, x, y, *w, *h, 1);
            }
            let main = if matches!(direction, LayoutDirection::Horizontal) { *w } else { *h };
            let cross = if matches!(direction, LayoutDirection::Horizontal) { *h } else { *w };
            cursor += main + spacing;
            cross_max = cross_max.max(cross);
        }

        let total_main = (cursor - spacing).max(0);
        let (pw, ph) = match direction {
            LayoutDirection::Horizontal => (total_main, cross_max),
            LayoutDirection::Vertical => (cross_max, total_main),
        };

        unsafe {
            MoveWindow(panel.hwnd, 0, 0, pw, ph, 1);
        }
        state::with_state(panel.hwnd, |s| s.natural_size = (pw, ph));

        panel
    }

    fn apply_style(&mut self, view: &Self::PlatformView, style: StyleProperty) {
        let StyleProperty(axis, value) = style;
        let hwnd = view.hwnd;

        state::with_state(hwnd, |s| match (&axis, &value) {
            (Axis::TextColor, StyleValue::Color(c)) => s.text_color = Some(state::to_colorref(goyda_utils::color::argb(*c))),
            (Axis::BackgroundColor, StyleValue::Color(c)) => s.background_color = Some(state::to_colorref(goyda_utils::color::argb(*c))),
            (Axis::BorderColor, StyleValue::Color(c)) => s.border_color = Some(state::to_colorref(goyda_utils::color::argb(*c))),
            (Axis::BorderWidth, v) => {
                if let Some(len) = goyda_utils::style::resolve_length(v) {
                    s.border_width = len as i32;
                }
            }
            (Axis::BorderRadius, v) => {
                if let Some(len) = goyda_utils::style::resolve_length(v) {
                    s.border_radius = len as i32;
                }
            }
            (Axis::Shadow, v) => {
                if let Some(len) = goyda_utils::style::resolve_length(v) {
                    s.shadow = len as i32;
                }
            }
            // Deliberately not handled here: `SetWindowLongPtrW`/
            // `SetLayeredWindowAttributes` can synchronously dispatch a
            // window message (e.g. `WM_STYLECHANGED`) back into this same
            // `HWND`'s `wndproc` before returning - which would try to
            // re-borrow `CONTROLS` while this closure's `with_state` borrow
            // is still held and panic. Applied below, once this closure (and
            // its borrow) has returned.
            (Axis::Opacity, StyleValue::Number(_)) => {}
            (Axis::FontSize, v) => {
                if let Some(len) = goyda_utils::style::resolve_length(v) {
                    s.font = Some(font::default_font(len as i32));
                }
            }
            (Axis::FontFamily, StyleValue::Asset(asset)) => {
                let size = DEFAULT_FONT_SIZE;
                if let Some(bytes) = asset.bytes() {
                    if let Some(f) = font::font_from_bytes(bytes, size) {
                        s.font = Some(f);
                    }
                } else if let Ok(bytes) = std::fs::read(
                    std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.join("assets").join(asset.path()))).unwrap_or_default(),
                ) {
                    if let Some(f) = font::font_from_bytes(&bytes, size) {
                        s.font = Some(f);
                    }
                }
            }
            (Axis::Padding(edge), v) => {
                if let Some(len) = goyda_utils::style::resolve_length(v) {
                    apply_edge(&mut s.padding, *edge, len as i32);
                }
            }
            (Axis::Margin(edge), v) => {
                if let Some(len) = goyda_utils::style::resolve_length(v) {
                    apply_edge(&mut s.margin, *edge, len as i32);
                }
            }
            _ => {}
        });

        if matches!(axis, Axis::Padding(_) | Axis::FontSize) {
            let (w, h) = measure_text_for(hwnd);
            if w > 0 || h > 0 {
                state::with_state(hwnd, |s| s.natural_size = (w, h));
            }
        }

        if let (Axis::Opacity, StyleValue::Number(alpha)) = (&axis, &value) {
            let a = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
            unsafe {
                let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED as isize);
                SetLayeredWindowAttributes(hwnd, 0, a, LWA_ALPHA);
            }
        }

        invalidate(hwnd);
    }
}

fn apply_edge(edges: &mut Edges, edge: Edge, v: i32) {
    match edge {
        Edge::All => {
            edges.left = v;
            edges.top = v;
            edges.right = v;
            edges.bottom = v;
        }
        Edge::Horizontal => {
            edges.left = v;
            edges.right = v;
        }
        Edge::Vertical => {
            edges.top = v;
            edges.bottom = v;
        }
        Edge::Top => edges.top = v,
        Edge::Right => edges.right = v,
        Edge::Bottom => edges.bottom = v,
        Edge::Left => edges.left = v,
    }
}

