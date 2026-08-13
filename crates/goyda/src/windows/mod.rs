pub mod backend;
mod font;
mod image;
mod layout;
mod paint;
mod state;

pub use backend::WindowsBackend;
pub use state::register_raw_hook;

use windows_sys::Win32::Foundation::HWND as WinHwnd;

/// Appends `ch` to the control's accumulated typed text (see
/// [`Event::TextChanged`](crate::core::events::Event::TextChanged)) and
/// returns the buffer's new contents. Used by the `text_watcher` listener's
/// `WM_CHAR` handling in [`crate::listeners`] - kept here instead of
/// exposing the whole `state` module just for this one field.
pub fn append_text_buffer(hwnd: WinHwnd, ch: char) -> String {
    let text = state::with_state(hwnd, |s| {
        s.text_buffer.push(ch);
        if let state::ControlKind::TextInput { text, .. } = &mut s.kind {
            *text = s.text_buffer.clone();
        }
        s.text_buffer.clone()
    })
    .unwrap_or_default();

    unsafe {
        windows_sys::Win32::Graphics::Gdi::InvalidateRect(hwnd, std::ptr::null(), 1);
    }

    text
}

/// Flips `hwnd`'s `ControlState::checked` and returns the new value. Used by
/// [`create_checkbox`](backend::WindowsBackend)/`create_switch`'s own
/// built-in click-to-toggle behavior, and (for any other control a developer
/// attaches `.on_checked_change(...)` to) by the `checked_change` listener's
/// `WM_LBUTTONUP` handling in [`crate::listeners`].
pub fn toggle_checked(hwnd: WinHwnd) -> bool {
    let now = state::with_state(hwnd, |s| {
        s.checked = !s.checked;
        s.checked
    })
    .unwrap_or(false);

    unsafe {
        windows_sys::Win32::Graphics::Gdi::InvalidateRect(hwnd, std::ptr::null(), 1);
    }

    now
}

/// Whether `hwnd` is a [`Checkbox`](crate::components::Checkbox)/
/// [`Switch`](crate::components::Switch) - these already toggle themselves
/// on click (see `create_checkbox`/`create_switch` in
/// [`backend::WindowsBackend`]), so the `checked_change` listener's
/// `WM_LBUTTONUP` hook only needs to *read* their state instead of also
/// toggling it (which would double-toggle back to the original value on
/// every click).
pub fn is_self_toggling(hwnd: WinHwnd) -> bool {
    state::with_state(hwnd, |s| {
        matches!(s.kind, state::ControlKind::Checkbox { .. } | state::ControlKind::Switch | state::ControlKind::RadioButton { .. })
    })
    .unwrap_or(false)
}

pub fn is_checked(hwnd: WinHwnd) -> bool {
    state::with_state(hwnd, |s| s.checked).unwrap_or(false)
}

/// Adds `hwnd` to `group`'s membership list - called once, at
/// `create_radio_button` time (see `windows/backend.rs`).
pub fn register_radio(hwnd: WinHwnd, group: &str) {
    state::RADIO_GROUPS.with(|g| {
        g.borrow_mut().entry(group.to_string()).or_default().push(hwnd);
    });
}

/// Selects `hwnd` (sets `checked = true`) and deselects every other member
/// of `group`, repainting all of them. Used by the intrinsic click
/// behavior `create_radio_button` registers - independent of whether the
/// app attaches `.on_checked_change(...)`, which only needs to *observe*
/// this (see `crate::listeners`'s `checked_change` arm and
/// [`is_self_toggling`]).
pub fn select_radio(hwnd: WinHwnd, group: &str) {
    let members = state::RADIO_GROUPS.with(|g| g.borrow().get(group).cloned().unwrap_or_default());
    for member in members {
        let selected = member == hwnd;
        let changed = state::with_state(member, |s| {
            let changed = s.checked != selected;
            s.checked = selected;
            changed
        })
        .unwrap_or(false);
        if changed {
            unsafe {
                windows_sys::Win32::Graphics::Gdi::InvalidateRect(member, std::ptr::null(), 1);
            }
        }
    }
}

/// Whether `hwnd` is a [`TextInput`](crate::components::TextInput) - these
/// already accumulate typed text themselves (see `create_text_input` in
/// [`backend::WindowsBackend`]), so the `text_watcher` listener's `WM_CHAR`
/// hook only needs to *read* the buffer instead of also appending to it
/// (which would insert every character twice).
pub fn is_text_input(hwnd: WinHwnd) -> bool {
    state::with_state(hwnd, |s| matches!(s.kind, state::ControlKind::TextInput { .. })).unwrap_or(false)
}

pub fn current_text_buffer(hwnd: WinHwnd) -> String {
    state::with_state(hwnd, |s| s.text_buffer.clone()).unwrap_or_default()
}

/// Removes the last character from the control's accumulated typed text (a
/// no-op on an already-empty buffer) and returns the buffer's new contents.
pub fn backspace_text_buffer(hwnd: WinHwnd) -> String {
    let text = state::with_state(hwnd, |s| {
        s.text_buffer.pop();
        if let state::ControlKind::TextInput { text, .. } = &mut s.kind {
            *text = s.text_buffer.clone();
        }
        s.text_buffer.clone()
    })
    .unwrap_or_default();

    unsafe {
        windows_sys::Win32::Graphics::Gdi::InvalidateRect(hwnd, std::ptr::null(), 1);
    }

    text
}

/// Sets a [`Progress`](crate::components::Progress) bar's value from a
/// click/drag's `x` position (window-relative, i.e. straight from a
/// `WM_LBUTTONDOWN`/`WM_MOUSEMOVE`'s `lParam`) as a fraction of the
/// control's *current* width - not its stored `natural_size`, which can be
/// stale once the enclosing stack has stretched the control to fill its
/// cross axis (see `windows/layout.rs`).
pub fn set_progress_from_x(hwnd: WinHwnd, x: i32) {
    let width = state::client_rect(hwnd).right.max(1);
    let fraction = (x as f32 / width as f32).clamp(0.0, 1.0);

    state::with_state(hwnd, |s| {
        if let state::ControlKind::Progress { value } = &mut s.kind {
            *value = fraction;
        }
    });

    unsafe {
        windows_sys::Win32::Graphics::Gdi::InvalidateRect(hwnd, std::ptr::null(), 1);
    }
}

pub fn current_progress(hwnd: WinHwnd) -> f32 {
    state::with_state(hwnd, |s| match &s.kind {
        state::ControlKind::Progress { value } => *value,
        _ => 0.0,
    })
    .unwrap_or(0.0)
}

/// Adjusts a [`ScrollView`](crate::components::ScrollView)'s scroll
/// position by `delta` pixels (clamped to `0..=(content size - viewport
/// size)`) and immediately re-flows its children to match - used by the
/// intrinsic `WM_MOUSEWHEEL` hook `create_scroll_view` registers (see
/// `windows/backend.rs`).
pub fn scroll_by(hwnd: WinHwnd, delta: i32) {
    let Some((direction, content_main)) = layout::content_main_size(hwnd) else { return };
    let rect = state::client_rect(hwnd);
    let viewport_main = match direction {
        crate::components::LayoutDirection::Horizontal => rect.right,
        crate::components::LayoutDirection::Vertical => rect.bottom,
    };
    let max_offset = (content_main - viewport_main).max(0);

    state::with_state(hwnd, |s| {
        s.scroll_offset = (s.scroll_offset + delta).clamp(0, max_offset);
    });

    layout::relayout(hwnd, rect.right, rect.bottom);
}

use std::cell::RefCell;

use windows_sys::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    InvalidateRect, RedrawWindow, UpdateWindow, COLOR_WINDOW, RDW_ALLCHILDREN, RDW_ERASE, RDW_INVALIDATE, RDW_UPDATENOW,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::{find_page, Component, Page};

const ROOT_CLASS_NAME: &str = "GoydaRoot\0";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

thread_local! {
    static ROOT: RefCell<Option<(HINSTANCE, HWND)>> = RefCell::new(None);
    /// Keeps the mounted page's `Component` (and the `Signal` subscriptions
    /// its reactive bindings hold) alive for as long as it's on screen -
    /// dropped and replaced on the next [`navigate`] call.
    static MOUNTED: RefCell<Option<Component>> = RefCell::new(None);
}

unsafe extern "system" fn root_wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            WM_SIZE => {
                let width = (lparam as u32 & 0xFFFF) as i32;
                let height = ((lparam as u32 >> 16) & 0xFFFF) as i32;
                let child = GetWindow(hwnd, GW_CHILD);
                if !child.is_null() {
                    layout::relayout(child, width, height);
                }
                InvalidateRect(hwnd, std::ptr::null(), 1);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

fn register_classes(hinstance: HINSTANCE) {
    let control_class_name = wide(state::CLASS_NAME.trim_end_matches('\0'));
    let root_class_name = wide(ROOT_CLASS_NAME.trim_end_matches('\0'));

    unsafe {
        let control_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(state::wndproc),
            hInstance: hinstance,
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            lpszClassName: control_class_name.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&control_class);

        let root_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(root_wndproc),
            hInstance: hinstance,
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_WINDOW + 1) as _,
            lpszClassName: root_class_name.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&root_class);
    }
}

/// Renders `page` into `root`. `fit_window` controls what happens to the
/// *window's own* size: `true` (initial launch) sizes the window to the
/// page's natural content size, like a desktop app popping up at a sensible
/// default size; `false` ([`navigate`]) leaves the window exactly as the
/// user last resized it and just re-flows the new page into that existing
/// client area - switching pages resizing the window back to some default
/// would undo a resize the user did on purpose.
fn mount(hinstance: HINSTANCE, root: HWND, page: &Page, fit_window: bool) {
    let component = (page.factory)();
    let mut backend = WindowsBackend::new(hinstance, root);
    let view = component.render(&mut backend);

    unsafe {
        if fit_window {
            let (w, h) = state::natural_size(view.hwnd);
            MoveWindow(view.hwnd, 0, 0, w, h, 1);

            let mut rect = windows_sys::Win32::Foundation::RECT { left: 0, top: 0, right: w, bottom: h };
            AdjustWindowRectEx(&mut rect, WS_OVERLAPPEDWINDOW, 0, 0);
            SetWindowPos(
                root,
                std::ptr::null_mut(),
                0,
                0,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOMOVE | SWP_NOZORDER,
            );
        } else {
            let client = state::client_rect(root);
            layout::relayout(view.hwnd, client.right - client.left, client.bottom - client.top);
        }

        InvalidateRect(root, std::ptr::null(), 1);
        UpdateWindow(root);
    }

    MOUNTED.with(|m| *m.borrow_mut() = Some(component));
}

/// Switches the mounted app to whichever `#[page(...)]` is registered for
/// `path` (see [`crate::find_page`]) - reparents/destroys nothing from the
/// previous page explicitly; each control is a genuine `HWND`, so
/// `DestroyWindow`ing the old root's children (done implicitly by replacing
/// them - see below) tears them down the same way closing any Win32 window
/// does.
pub fn navigate(path: &str) {
    let Some(page) = find_page(path) else {
        #[cfg(debug_assertions)]
        eprintln!("goyda(windows): navigate(\"{path}\") - no #[page(...)] registered for that route");
        return;
    };

    let Some((hinstance, root)) = ROOT.with(|r| *r.borrow()) else { return };

    unsafe {
        // Suppress repainting for the whole teardown-then-rebuild sequence -
        // without this, the moment between `DestroyWindow`ing the old page's
        // controls and the new page's controls appearing is a real (if
        // brief) frame where `root` has no children and its own plain
        // background shows through, flashing blank before the new page
        // shows up.
        SendMessageW(root, WM_SETREDRAW, 0, 0);

        let mut child = GetWindow(root, GW_CHILD);
        while !child.is_null() {
            let next = GetWindow(child, GW_HWNDNEXT);
            DestroyWindow(child);
            child = next;
        }
    }

    mount(hinstance, root, page, false);

    unsafe {
        SendMessageW(root, WM_SETREDRAW, 1, 0);
        RedrawWindow(
            root,
            std::ptr::null(),
            std::ptr::null_mut(),
            RDW_ERASE | RDW_INVALIDATE | RDW_ALLCHILDREN | RDW_UPDATENOW,
        );
    }
}

/// A double-clicked `.exe` has no console attached to print panic messages
/// to, so they'd otherwise vanish silently - this puts them in front of the
/// user as a message box instead. Running from a terminal still also prints
/// the usual panic output there (this hook doesn't replace it).
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default_hook(info);
        let text = wide(&info.to_string());
        let title = wide("goyda: unexpected error");
        unsafe {
            MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR);
        }
    }));
}

/// Runs the app: registers goyda's window classes, creates the top-level
/// window, mounts the initial `#[page("/")]`, and pumps the Win32 message
/// loop until the window closes. Called from the small `fn main()` the
/// `goy` CLI generates for a windows build - see
/// `goyda-cli/src/targets/windows`.
pub fn run() {
    install_panic_hook();

    let hinstance: HINSTANCE = unsafe { GetModuleHandleW(std::ptr::null()) as HINSTANCE };
    register_classes(hinstance);

    let root = unsafe {
        CreateWindowExW(
            0,
            wide(ROOT_CLASS_NAME.trim_end_matches('\0')).as_ptr(),
            wide("goyda").as_ptr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            800,
            600,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null(),
        )
    };

    ROOT.with(|r| *r.borrow_mut() = Some((hinstance, root)));

    let page = find_page("/").expect("goyda(windows): no #[page(\"/\")] registered");
    mount(hinstance, root, page, true);

    unsafe {
        ShowWindow(root, SW_SHOW);
        UpdateWindow(root);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
