use windows_sys::Win32::Foundation::{HINSTANCE, HWND};
use windows_sys::Win32::Graphics::Gdi::{GetDC, GetTextExtentPoint32W, InvalidateRect, ReleaseDC, SelectObject, HFONT};
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
                    _ => {}
                });
                let (w, h) = measure_text_for(view.hwnd);
                state::with_state(view.hwnd, |s| s.natural_size = (w, h));
                invalidate(view.hwnd);
            }
        }
    }
}

fn measure_text_for(hwnd: HWND) -> (i32, i32) {
    state::with_state(hwnd, |s| {
        let is_button = matches!(s.kind, ControlKind::Button(_));
        let text = match &s.kind {
            ControlKind::Text(t) | ControlKind::Button(t) => t.clone(),
            _ => return (0, 0),
        };
        let f = s.font.unwrap_or_else(|| font::default_font(DEFAULT_FONT_SIZE));
        let (w, h) = measure_text(&text, f);
        let pad = if is_button { &BUTTON_PADDING } else { &s.padding };
        (w + pad.left + pad.right, h + pad.top + pad.bottom)
    })
    .unwrap_or((0, 0))
}

fn invalidate(hwnd: HWND) {
    unsafe {
        InvalidateRect(hwnd, std::ptr::null(), 1);
    }
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

