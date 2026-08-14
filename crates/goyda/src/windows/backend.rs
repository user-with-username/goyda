use windows_sys::Win32::Foundation::{HINSTANCE, HWND, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    GetDC, GetTextExtentPoint32W, HFONT, InvalidateRect, ReleaseDC, SelectObject,
    SetTextCharacterExtra,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, SetFocus};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::components::{Asset, Axis, Edge, LayoutDirection, StyleProperty, StyleValue};
use crate::core::events::Update;
use crate::core::{Backend, BackendUpdater};

use super::font;
use super::image::decode_to_bitmap;
use super::layout;
use super::state::{self, ControlKind, ControlState, Edges};

const DEFAULT_FONT_SIZE: i32 = 16;
const BUTTON_PADDING: Edges = Edges {
    left: 16,
    top: 10,
    right: 16,
    bottom: 10,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn measure_text(text: &str, font: HFONT) -> (i32, i32) {
    measure_text_with_spacing(text, font, 0)
}

fn measure_text_with_spacing(text: &str, font: HFONT, letter_spacing: i32) -> (i32, i32) {
    unsafe {
        let hdc = GetDC(std::ptr::null_mut());
        let old = SelectObject(hdc, font as _);
        if letter_spacing != 0 {
            SetTextCharacterExtra(hdc, letter_spacing);
        }
        let w = wide(text);
        let mut size = windows_sys::Win32::Foundation::SIZE::default();
        GetTextExtentPoint32W(hdc, w.as_ptr(), (w.len() - 1) as i32, &mut size);
        SelectObject(hdc, old);
        ReleaseDC(std::ptr::null_mut(), hdc);
        (size.cx, size.cy)
    }
}

/// A handle to a mounted control on Windows.
#[derive(Clone, Copy)]
pub struct WindowsView {
    pub hwnd: HWND,
}

/// Applies reactive updates to controls on Windows.
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

const TEXT_INPUT_MIN_WIDTH: i32 = 150;
const TEXTAREA_MIN_WIDTH: i32 = 250;
const TEXTAREA_MIN_HEIGHT: i32 = 80;
const CHECKBOX_BOX_SIZE: i32 = 16;
const SWITCH_SIZE: (i32, i32) = (40, 22);
const PROGRESS_HEIGHT: i32 = 8;
const PROGRESS_DEFAULT_WIDTH: i32 = 200;
const DIVIDER_THICKNESS: i32 = 1;

fn measure_text_for(hwnd: HWND) -> (i32, i32) {
    state::with_state(hwnd, |s| {
        let is_button = matches!(s.kind, ControlKind::Button(_));
        let multiline = matches!(
            s.kind,
            ControlKind::TextInput {
                multiline: true,
                ..
            }
        );
        let is_input = matches!(s.kind, ControlKind::TextInput { .. });
        let text = match &s.kind {
            ControlKind::Text(t) | ControlKind::Button(t) => t.clone(),
            ControlKind::TextInput {
                text, placeholder, ..
            } => {
                if text.is_empty() {
                    placeholder.clone()
                } else {
                    text.clone()
                }
            }
            _ => return (0, 0),
        };
        let f = s
            .font
            .unwrap_or_else(|| font::default_font(DEFAULT_FONT_SIZE));
        let (w, h) = measure_text_with_spacing(&text, f, s.letter_spacing);
        let pad = if is_button {
            &BUTTON_PADDING
        } else {
            &s.padding
        };
        let total_w = w + pad.left + pad.right;
        let total_h = h + pad.top + pad.bottom;
        if multiline {
            (
                total_w.max(TEXTAREA_MIN_WIDTH),
                total_h.max(TEXTAREA_MIN_HEIGHT),
            )
        } else if is_input {
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

fn register_click_to_toggle(hwnd: HWND) {
    state::register_raw_hook(
        hwnd,
        std::rc::Rc::new(move |msg, _wparam, _lparam| {
            if msg == WM_LBUTTONUP {
                crate::windows::toggle_checked(hwnd);
            }
        }),
    );
}

/// The Windows rendering backend, mounting components as native `HWND`
/// controls under `parent`.
pub struct WindowsBackend {
    pub hinstance: HINSTANCE,
    pub parent: HWND,
}

impl WindowsBackend {
    /// Creates a backend that mounts controls as children of `parent`.
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
                // `WS_CLIPSIBLINGS`: without it, a sibling's own `WM_PAINT`
                // fill can paint straight over a *higher z-order* sibling
                // that overlaps it (nothing before `Overlay` needed this -
                // ordinary `Panel`/`ScrollView` children never overlap, but
                // `Overlay` children intentionally do).
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
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

    fn build_stack_control(
        &mut self,
        kind: ControlKind,
        direction: LayoutDirection,
        spacing: i32,
        children: Vec<WindowsView>,
    ) -> WindowsView {
        let panel = self.create_control(kind);

        for child in &children {
            unsafe {
                SetParent(child.hwnd, panel.hwnd);
            }
        }

        let sizes: Vec<(i32, i32)> = children
            .iter()
            .map(|c| state::natural_size(c.hwnd))
            .collect();
        let mut total_main = 0i32;
        let mut cross_max = 0i32;
        for (i, (w, h)) in sizes.iter().enumerate() {
            let main = if matches!(direction, LayoutDirection::Horizontal) {
                *w
            } else {
                *h
            };
            let cross = if matches!(direction, LayoutDirection::Horizontal) {
                *h
            } else {
                *w
            };
            total_main += main + if i > 0 { spacing } else { 0 };
            cross_max = cross_max.max(cross);
        }

        let (pw, ph) = match direction {
            LayoutDirection::Horizontal => (total_main, cross_max),
            LayoutDirection::Vertical => (cross_max, total_main),
        };

        unsafe {
            MoveWindow(panel.hwnd, 0, 0, pw.max(0), ph.max(0), 1);
        }
        state::with_state(panel.hwnd, |s| s.natural_size = (pw, ph));

        layout::relayout(panel.hwnd, pw, ph);

        panel
    }

    fn build_text_input(
        &mut self,
        placeholder: &str,
        initial_text: &str,
        multiline: bool,
    ) -> WindowsView {
        let view = self.create_control(ControlKind::TextInput {
            text: initial_text.to_string(),
            placeholder: placeholder.to_string(),
            multiline,
        });
        state::with_state(view.hwnd, |s| {
            s.text_buffer = initial_text.to_string();
            s.background_color = Some(state::to_colorref(0xFFFFFFFF));
            s.border_color = Some(state::to_colorref(0xFF969696));
            s.border_width = 1;
            s.border_radius = 2;
            s.padding = Edges {
                left: 8,
                top: 4,
                right: 8,
                bottom: 4,
            };
        });
        let (w, h) = measure_text_for(view.hwnd);
        state::with_state(view.hwnd, |s| s.natural_size = (w, h));

        // Built-in interactivity, matching what a real `<input>`/`<textarea>`/
        // `EditText` gives for free on the other two backends: click to
        // focus (nothing else on this backend ever calls `SetFocus`, so
        // without this a freshly-created field could never receive
        // `WM_CHAR` at all), then accumulate typed characters (Enter as a
        // newline when `multiline`, backspace always) straight into this
        // control's own buffer - independent of whether the app attaches
        // `.on_text_changed(...)`, which only needs to *observe* this, not
        // drive it (see `crate::listeners`'s `text_watcher` arm).
        let hwnd = view.hwnd;
        state::register_raw_hook(
            hwnd,
            std::rc::Rc::new(move |msg, wparam, _lparam| unsafe {
                match msg {
                    WM_LBUTTONDOWN => {
                        SetFocus(hwnd);
                    }
                    WM_CHAR => {
                        if wparam == 0x08 {
                            crate::windows::backspace_text_buffer(hwnd);
                        } else if multiline && (wparam == 0x0D || wparam == 0x0A) {
                            crate::windows::append_text_buffer(hwnd, '\n');
                        } else if let Some(ch) = char::from_u32(wparam as u32) {
                            if !ch.is_control() {
                                crate::windows::append_text_buffer(hwnd, ch);
                            }
                        }
                    }
                    _ => {}
                }
            }),
        );

        view
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
            Some((bmp, w, h)) => (
                ControlKind::Image {
                    bitmap: Some(bmp),
                    width: w,
                    height: h,
                },
                (w, h),
            ),
            None => (
                ControlKind::Image {
                    bitmap: None,
                    width: 0,
                    height: 0,
                },
                (0, 0),
            ),
        };
        let view = self.create_control(kind);
        state::with_state(view.hwnd, |s| s.natural_size = size);
        view
    }

    fn create_text_input(&mut self, placeholder: &str, initial_text: &str) -> Self::PlatformView {
        self.build_text_input(placeholder, initial_text, false)
    }

    fn create_textarea(&mut self, placeholder: &str, initial_text: &str) -> Self::PlatformView {
        self.build_text_input(placeholder, initial_text, true)
    }

    fn create_checkbox(&mut self, label: &str, checked: bool) -> Self::PlatformView {
        let view = self.create_control(ControlKind::Checkbox {
            label: label.to_string(),
        });
        let font = font::default_font(DEFAULT_FONT_SIZE);
        let (label_w, label_h) = if label.is_empty() {
            (0, 0)
        } else {
            measure_text(label, font)
        };
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

    fn create_radio_button(
        &mut self,
        group: &str,
        label: &str,
        selected: bool,
    ) -> Self::PlatformView {
        let view = self.create_control(ControlKind::RadioButton {
            label: label.to_string(),
        });
        let font = font::default_font(DEFAULT_FONT_SIZE);
        let (label_w, label_h) = if label.is_empty() {
            (0, 0)
        } else {
            measure_text(label, font)
        };
        let gap = if label.is_empty() { 0 } else { 8 };
        let w = CHECKBOX_BOX_SIZE + gap + label_w;
        let h = CHECKBOX_BOX_SIZE.max(label_h);
        state::with_state(view.hwnd, |s| {
            s.checked = selected;
            s.natural_size = (w, h);
        });

        crate::windows::register_radio(view.hwnd, group);
        if selected {
            crate::windows::select_radio(view.hwnd, group);
        }

        let hwnd = view.hwnd;
        let group_owned = group.to_string();
        state::register_raw_hook(
            hwnd,
            std::rc::Rc::new(move |msg, _wparam, _lparam| {
                if msg == WM_LBUTTONUP {
                    crate::windows::select_radio(hwnd, &group_owned);
                }
            }),
        );

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
        let view = self.create_control(ControlKind::Progress {
            value: value.clamp(0.0, 1.0),
        });
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
        state::register_raw_hook(
            hwnd,
            std::rc::Rc::new(move |msg, wparam, lparam| unsafe {
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
            }),
        );

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

    fn create_stack(
        &mut self,
        direction: LayoutDirection,
        spacing: i32,
        children: Vec<Self::PlatformView>,
    ) -> Self::PlatformView {
        self.build_stack_control(
            ControlKind::Panel {
                direction,
                spacing,
                children: children.iter().map(|c| c.hwnd).collect(),
            },
            direction,
            spacing,
            children,
        )
    }

    fn create_scroll_view(
        &mut self,
        direction: LayoutDirection,
        spacing: i32,
        children: Vec<Self::PlatformView>,
    ) -> Self::PlatformView {
        let view = self.build_stack_control(
            ControlKind::ScrollView {
                direction,
                spacing,
                children: children.iter().map(|c| c.hwnd).collect(),
            },
            direction,
            spacing,
            children,
        );

        // No native scrollable widget exists in this backend (see
        // `windows/state.rs`'s `ControlKind`), so scrolling is just this
        // control's own main-axis offset (`ControlState::scroll_offset`),
        // nudged by the mouse wheel and applied by `relayout` (see
        // `crate::windows::scroll_by`) - child `HWND`s scrolled outside
        // this control's own bounds are clipped for free, the same way any
        // child window is always clipped to its parent's client area.
        let hwnd = view.hwnd;
        state::register_raw_hook(
            hwnd,
            std::rc::Rc::new(move |msg, wparam, _lparam| {
                if msg == WM_MOUSEWHEEL {
                    let notches = ((wparam >> 16) as i16 as i32) / WHEEL_DELTA as i32;
                    crate::windows::scroll_by(hwnd, -notches * 40);
                }
            }),
        );

        view
    }

    fn create_overlay(&mut self, children: Vec<Self::PlatformView>) -> Self::PlatformView {
        let overlay = self.create_control(ControlKind::Overlay {
            children: children.iter().map(|c| c.hwnd).collect(),
        });

        for child in &children {
            unsafe {
                SetParent(child.hwnd, overlay.hwnd);
            }
        }

        // Painted back-to-front, so a higher `z_index` (or, for ties, a
        // later position in `children`) ends up drawn last - i.e. on top -
        // matching how DOM/view child order already works as the default
        // stacking rule when `z_index` is left unset.
        let mut ordered: Vec<(i32, HWND)> = children
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let z = state::with_state(c.hwnd, |s| s.z_index).unwrap_or(0);
                (z * (children.len() as i32 + 1) + i as i32, c.hwnd)
            })
            .collect();
        ordered.sort_by_key(|(key, _)| *key);

        let mut max_w = 0;
        let mut max_h = 0;
        for (_, hwnd) in &ordered {
            let (x, y) = state::with_state(*hwnd, |s| (s.offset_x, s.offset_y)).unwrap_or((0, 0));
            let (w, h) = state::natural_size(*hwnd);
            unsafe {
                MoveWindow(*hwnd, x, y, w.max(0), h.max(0), 1);
                SetWindowPos(*hwnd, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            }
            max_w = max_w.max(x + w);
            max_h = max_h.max(y + h);
        }

        unsafe {
            MoveWindow(overlay.hwnd, 0, 0, max_w.max(0), max_h.max(0), 1);
        }
        state::with_state(overlay.hwnd, |s| s.natural_size = (max_w, max_h));

        overlay
    }

    fn apply_style(&mut self, view: &Self::PlatformView, style: StyleProperty) {
        let StyleProperty(axis, value) = style;
        let hwnd = view.hwnd;

        state::with_state(hwnd, |s| match (&axis, &value) {
            (Axis::TextColor, StyleValue::Color(c)) => {
                s.text_color = Some(state::to_colorref(goyda_utils::color::argb(*c)))
            }
            (Axis::BackgroundColor, StyleValue::Color(c)) => {
                s.background_color = Some(state::to_colorref(goyda_utils::color::argb(*c)))
            }
            (Axis::BorderColor, StyleValue::Color(c)) => {
                s.border_color = Some(state::to_colorref(goyda_utils::color::argb(*c)))
            }
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
                    s.font_size = len as i32;
                    s.font = Some(font::styled_font(
                        s.font_size,
                        s.bold,
                        s.italic,
                        s.underline,
                        s.strikethrough,
                    ));
                }
            }
            // `.bold()`/`.italic()`/`.underline()`/`.strikethrough()`
            // rebuild from the tracked `(size, bold, italic, underline,
            // strikethrough)` tuple - see `ControlState::font_size`. This
            // means they override (rather than compose with) a custom
            // `.font(asset)` family, since a custom family's
            // weight/slant/decoration isn't tracked here - a known
            // limitation, not a bug.
            (Axis::FontWeight, StyleValue::Bool(bold)) => {
                s.bold = *bold;
                s.font = Some(font::styled_font(
                    s.font_size,
                    s.bold,
                    s.italic,
                    s.underline,
                    s.strikethrough,
                ));
            }
            (Axis::FontStyle, StyleValue::Bool(italic)) => {
                s.italic = *italic;
                s.font = Some(font::styled_font(
                    s.font_size,
                    s.bold,
                    s.italic,
                    s.underline,
                    s.strikethrough,
                ));
            }
            (Axis::Underline, StyleValue::Bool(underline)) => {
                s.underline = *underline;
                s.font = Some(font::styled_font(
                    s.font_size,
                    s.bold,
                    s.italic,
                    s.underline,
                    s.strikethrough,
                ));
            }
            (Axis::Strikethrough, StyleValue::Bool(strikethrough)) => {
                s.strikethrough = *strikethrough;
                s.font = Some(font::styled_font(
                    s.font_size,
                    s.bold,
                    s.italic,
                    s.underline,
                    s.strikethrough,
                ));
            }
            (Axis::TextAlign, StyleValue::Align(a)) => s.text_align = *a,
            (Axis::Width, v) => {
                if let Some(len) = goyda_utils::style::resolve_length(v) {
                    s.explicit_width = Some(len as i32);
                }
            }
            (Axis::Height, v) => {
                if let Some(len) = goyda_utils::style::resolve_length(v) {
                    s.explicit_height = Some(len as i32);
                }
            }
            (Axis::AlignItems, StyleValue::Align(a)) => s.align_items = *a,
            (Axis::JustifyContent, StyleValue::Align(a)) => s.justify_content = *a,
            (Axis::LineHeight, v) => {
                if let Some(len) = goyda_utils::style::resolve_length(v) {
                    s.line_height = Some(len as i32);
                }
            }
            (Axis::LetterSpacing, v) => {
                if let Some(len) = goyda_utils::style::resolve_length(v) {
                    s.letter_spacing = len as i32;
                }
            }
            (Axis::TextOverflowEllipsis, StyleValue::Bool(v)) => s.ellipsis = *v,
            // `Axis::Clip` is a no-op here - see its doc comment: child
            // `HWND`s are already clipped to their parent's bounds
            // unconditionally, this backend has no way to opt back into
            // `overflow: visible` even if asked.
            (Axis::Clip, StyleValue::Bool(_)) => {}
            (Axis::ShadowColor, StyleValue::Color(c)) => {
                s.shadow_color = Some(state::to_colorref(goyda_utils::color::argb(*c)));
            }
            (Axis::OffsetX, v) => {
                if let Some(len) = goyda_utils::style::resolve_length(v) {
                    s.offset_x = len as i32;
                }
            }
            (Axis::OffsetY, v) => {
                if let Some(len) = goyda_utils::style::resolve_length(v) {
                    s.offset_y = len as i32;
                }
            }
            (Axis::ZIndex, StyleValue::Number(z)) => s.z_index = *z as i32,
            (Axis::FontFamily, StyleValue::Asset(asset)) => {
                let size = DEFAULT_FONT_SIZE;
                if let Some(bytes) = asset.bytes() {
                    if let Some(f) = font::font_from_bytes(bytes, size) {
                        s.font = Some(f);
                    }
                } else if let Ok(bytes) = std::fs::read(
                    std::env::current_exe()
                        .ok()
                        .and_then(|p| p.parent().map(|p| p.join("assets").join(asset.path())))
                        .unwrap_or_default(),
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

        if matches!(
            axis,
            Axis::Padding(_)
                | Axis::FontSize
                | Axis::FontWeight
                | Axis::FontStyle
                | Axis::LetterSpacing
        ) {
            let (w, h) = measure_text_for(hwnd);
            if w > 0 || h > 0 {
                state::with_state(hwnd, |s| s.natural_size = (w, h));
            }
        }

        if matches!(axis, Axis::Width | Axis::Height) {
            let (w, h) = state::natural_size(hwnd);
            unsafe {
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    0,
                    0,
                    w.max(0),
                    h.max(0),
                    SWP_NOMOVE | SWP_NOZORDER,
                );
            }
        }

        if matches!(axis, Axis::AlignItems | Axis::JustifyContent) {
            let rect = state::client_rect(hwnd);
            layout::relayout(hwnd, rect.right, rect.bottom);
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
