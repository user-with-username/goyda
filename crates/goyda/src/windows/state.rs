//! goyda owns exactly one Win32 window class ("GoydaControl") and uses it
//! for every control - text, buttons, images, and stack panels alike.
//! Content and style live in a side table keyed by `HWND` rather than in
//! native control state, and a single shared `WndProc` paints from that
//! table and forwards every message to whatever raw hooks a listener (see
//! [`crate::listeners`]) registered on that `HWND`. This is what lets one
//! event-dispatch mechanism serve click, focus, and text events uniformly,
//! the same way the web backend dispatches DOM events straight at the
//! element and Android dispatches JNI callbacks straight at the view -
//! nothing is routed through a parent window's `WM_COMMAND`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use windows_sys::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{HBITMAP, HFONT};

use crate::components::{Align, LayoutDirection};

pub const CLASS_NAME: &str = "GoydaControl\0";

#[derive(Clone)]
pub enum ControlKind {
    Text(String),
    Button(String),
    Image { bitmap: Option<HBITMAP>, width: i32, height: i32 },
    /// `children` is kept in the order `create_stack` built them, so a
    /// window resize can re-run layout later (see
    /// [`crate::windows::layout::relayout`]) without needing to re-render
    /// the component tree - only the initial (wrap-content) sizing happens
    /// during `create_stack` itself.
    Panel { direction: LayoutDirection, spacing: i32, children: Vec<HWND> },
    /// Structurally identical to `Panel` - kept as its own variant (rather
    /// than a bool flag on `Panel`) so `relayout` (see
    /// `crate::windows::layout`) can pattern-match both together with an
    /// or-pattern while everything else that only cares about "is this a
    /// panel" (e.g. `paint.rs`) can still tell them apart.
    ScrollView { direction: LayoutDirection, spacing: i32, children: Vec<HWND> },
    /// `text` holds what's actually been typed (kept in sync with
    /// [`ControlState::text_buffer`] by the `text_watcher` listener's
    /// `WM_CHAR` hook), `placeholder` is only drawn when `text` is empty.
    /// `multiline` distinguishes a [`TextInput`](crate::components::TextInput)
    /// (`false`) from a [`Textarea`](crate::components::Textarea) (`true`) -
    /// same control kind either way, since the only real differences are
    /// whether Enter inserts a newline and how the text wraps/sizes.
    TextInput { text: String, placeholder: String, multiline: bool },
    Checkbox { label: String },
    /// Which group (see [`RADIO_GROUPS`]) is tracked there, not here.
    RadioButton { label: String },
    Switch,
    /// `0.0..=1.0`.
    Progress { value: f32 },
    Spacer,
    Divider,
    /// No linear flow like `Panel` - every child is positioned at its own
    /// `ControlState::offset_x`/`offset_y`, stacked by
    /// `ControlState::z_index` (`position: absolute` in CSS terms). See
    /// [`crate::components::Overlay`].
    Overlay { children: Vec<HWND> },
}

#[derive(Clone, Default)]
pub struct Edges {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

pub struct ControlState {
    pub kind: ControlKind,
    pub text_color: Option<COLORREF>,
    pub background_color: Option<COLORREF>,
    pub border_color: Option<COLORREF>,
    pub border_width: i32,
    pub border_radius: i32,
    pub shadow: i32,
    pub font: Option<HFONT>,
    pub padding: Edges,
    pub margin: Edges,
    /// The control's own preferred size, ignoring layout - measured text
    /// extent for `Text`/`Button`, the decoded bitmap's dimensions for
    /// `Image`, and the summed/max of children for `Panel`. `create_stack`
    /// reads every child's natural size (already computed bottom-up, since
    /// children always render before their parent) to lay them out, and
    /// stores its own resulting total back into this same field so the
    /// composition works at any nesting depth.
    pub natural_size: (i32, i32),
    pub text_buffer: String,
    /// Toggled state for `Checkbox`/`Switch` controls, flipped in-place by
    /// the `checked_change` listener's windows hook (see
    /// `crate::listeners`) so painting can reflect it.
    pub checked: bool,
    /// `Axis::Width`/`Axis::Height` overrides - read by [`natural_size`],
    /// so anything that already asks for a control's natural size (layout,
    /// paint) gets the override for free with no separate call site.
    pub explicit_width: Option<i32>,
    pub explicit_height: Option<i32>,
    pub bold: bool,
    pub italic: bool,
    /// Tracked alongside `font` so `Axis::FontWeight`/`Axis::FontStyle` can
    /// rebuild the right `(size, bold, italic)` font without needing to
    /// already know the current size.
    pub font_size: i32,
    pub text_align: Align,
    /// Only meaningful when `kind` is `Panel` - cross-axis alignment of its
    /// children (see [`crate::windows::layout::relayout`]).
    pub align_items: Align,
    /// Only meaningful when `kind` is `Panel` - main-axis distribution of
    /// its children (see [`crate::windows::layout::relayout`]).
    pub justify_content: Align,
    /// Only meaningful when `kind` is `ScrollView` - current scroll
    /// position along the main axis, in pixels, clamped to
    /// `0..=(content size - viewport size)` by
    /// [`crate::windows::scroll_by`].
    pub scroll_offset: i32,
    pub line_height: Option<i32>,
    pub letter_spacing: i32,
    pub underline: bool,
    pub strikethrough: bool,
    pub ellipsis: bool,
    pub shadow_color: Option<COLORREF>,
    /// Only meaningful on a direct child of an `Overlay` - see
    /// `ControlKind::Overlay`.
    pub offset_x: i32,
    pub offset_y: i32,
    pub z_index: i32,
}

impl ControlState {
    pub fn new(kind: ControlKind) -> Self {
        Self {
            kind,
            text_color: None,
            background_color: None,
            border_color: None,
            border_width: 0,
            border_radius: 0,
            shadow: 0,
            font: None,
            padding: Edges::default(),
            margin: Edges::default(),
            natural_size: (0, 0),
            text_buffer: String::new(),
            checked: false,
            explicit_width: None,
            explicit_height: None,
            bold: false,
            italic: false,
            font_size: 16,
            text_align: Align::Start,
            align_items: Align::Stretch,
            justify_content: Align::Start,
            scroll_offset: 0,
            line_height: None,
            letter_spacing: 0,
            underline: false,
            strikethrough: false,
            ellipsis: false,
            shadow_color: None,
            offset_x: 0,
            offset_y: 0,
            z_index: 0,
        }
    }
}

pub type RawHook = Rc<dyn Fn(u32, WPARAM, LPARAM)>;

#[derive(Default)]
pub struct HwndData {
    pub state: Option<ControlState>,
    pub hooks: Vec<RawHook>,
}

thread_local! {
    /// Every control lives on the single UI thread that owns the message
    /// loop, so a thread-local table (no locking) is enough.
    pub static CONTROLS: RefCell<HashMap<isize, HwndData>> = RefCell::new(HashMap::new());
    /// Group name -> every `RadioButton` `HWND` created with that group -
    /// no native "radio group" container exists in this backend (see
    /// `ControlKind`), so mutual exclusion is done by hand: selecting one
    /// unchecks every other `HWND` in the same `Vec` (see
    /// `crate::windows::select_radio`).
    pub static RADIO_GROUPS: RefCell<HashMap<String, Vec<HWND>>> = RefCell::new(HashMap::new());
}

pub fn key(hwnd: HWND) -> isize {
    hwnd as isize
}

pub fn with_state<R>(hwnd: HWND, f: impl FnOnce(&mut ControlState) -> R) -> Option<R> {
    CONTROLS.with(|c| c.borrow_mut().get_mut(&key(hwnd)).and_then(|d| d.state.as_mut()).map(f))
}

pub fn register_raw_hook(hwnd: HWND, hook: RawHook) {
    CONTROLS.with(|c| {
        c.borrow_mut().entry(key(hwnd)).or_default().hooks.push(hook);
    });
}

pub fn insert_state(hwnd: HWND, state: ControlState) {
    CONTROLS.with(|c| {
        c.borrow_mut().entry(key(hwnd)).or_default().state = Some(state);
    });
}

pub fn remove(hwnd: HWND) {
    CONTROLS.with(|c| {
        c.borrow_mut().remove(&key(hwnd));
    });
}

pub fn natural_size(hwnd: HWND) -> (i32, i32) {
    with_state(hwnd, |s| (s.explicit_width.unwrap_or(s.natural_size.0), s.explicit_height.unwrap_or(s.natural_size.1))).unwrap_or((0, 0))
}

pub fn to_colorref(argb: u32) -> COLORREF {
    let r = (argb >> 16) & 0xFF;
    let g = (argb >> 8) & 0xFF;
    let b = argb & 0xFF;
    (b << 16) | (g << 8) | r
}

pub unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    // Hooks run for every message (mouse/keyboard/focus events a listener
    // registered for), *before* the built-in handling below - a hook never
    // needs to suppress default painting/erasing, so there's nothing to
    // short-circuit here.
    let hooks: Vec<RawHook> =
        CONTROLS.with(|c| c.borrow().get(&key(hwnd)).map(|d| d.hooks.clone()).unwrap_or_default());
    for hook in &hooks {
        hook(msg, wparam, lparam);
    }

    unsafe {
        match msg {
            WM_ERASEBKGND => 1,
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                crate::windows::paint::paint_control(hwnd, hdc);
                EndPaint(hwnd, &ps);
                0
            }
            WM_DESTROY => {
                remove(hwnd);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

pub fn client_rect(hwnd: HWND) -> RECT {
    unsafe {
        let mut r = RECT::default();
        windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut r);
        r
    }
}
