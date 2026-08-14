pub mod backend;
mod font;
mod image;
mod layout;
mod paint;
mod state;

pub use backend::WindowsBackend;

/// Registers a raw window-message hook on a control, invoked with every
/// message it receives.
pub use state::register_raw_hook;

use windows_sys::Win32::Foundation::HWND as WinHwnd;

/// Appends `ch` to a text control's contents and returns the updated text.
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

/// Toggles a checkbox, switch, or radio button's checked state and returns
/// the new value.
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

/// Whether `hwnd` is a control that already toggles its own checked state
/// on click (a checkbox, switch, or radio button).
pub fn is_self_toggling(hwnd: WinHwnd) -> bool {
    state::with_state(hwnd, |s| {
        matches!(s.kind, state::ControlKind::Checkbox { .. } | state::ControlKind::Switch | state::ControlKind::RadioButton { .. })
    })
    .unwrap_or(false)
}

/// Returns whether `hwnd` is currently checked.
pub fn is_checked(hwnd: WinHwnd) -> bool {
    state::with_state(hwnd, |s| s.checked).unwrap_or(false)
}

/// Adds `hwnd` to the named radio button group.
pub fn register_radio(hwnd: WinHwnd, group: &str) {
    state::RADIO_GROUPS.with(|g| {
        g.borrow_mut().entry(group.to_string()).or_default().push(hwnd);
    });
}

/// Selects `hwnd` and deselects every other member of its radio button
/// group.
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

/// Returns whether `hwnd` is a text input control.
pub fn is_text_input(hwnd: WinHwnd) -> bool {
    state::with_state(hwnd, |s| matches!(s.kind, state::ControlKind::TextInput { .. })).unwrap_or(false)
}

/// Returns a text control's current contents.
pub fn current_text_buffer(hwnd: WinHwnd) -> String {
    state::with_state(hwnd, |s| s.text_buffer.clone()).unwrap_or_default()
}

/// Removes the last character from a text control's contents (a no-op if
/// already empty) and returns the updated text.
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

/// Sets a progress bar's value from a click/drag's window-relative `x`
/// position, as a fraction of the control's width.
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

/// Returns a progress bar's current value, from `0.0` to `1.0`.
pub fn current_progress(hwnd: WinHwnd) -> f32 {
    state::with_state(hwnd, |s| match &s.kind {
        state::ControlKind::Progress { value } => *value,
        _ => 0.0,
    })
    .unwrap_or(0.0)
}

/// Scrolls a scroll view by `delta` pixels, clamped to its content bounds.
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
use windows_sys::Win32::System::DataExchange::COPYDATASTRUCT;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, LoadLibraryW};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::{find_page, Component, LayoutDirection, Page};

const ROOT_CLASS_NAME: &str = "GoydaRoot\0";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

thread_local! {
    static ROOT: RefCell<Option<(HINSTANCE, HWND)>> = RefCell::new(None);
    static MOUNTED: RefCell<Option<Component>> = RefCell::new(None);
    static CURRENT_PATH: RefCell<String> = RefCell::new(String::from("/"));
}

fn detect_theme_mode() -> crate::core::theme::ThemeMode {
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};

    let subkey = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let value_name = wide("AppsUseLightTheme");
    let mut data: u32 = 1;
    let mut size: u32 = std::mem::size_of::<u32>() as u32;

    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            value_name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            &mut data as *mut u32 as *mut _,
            &mut size,
        )
    };

    if status == 0 && data == 0 {
        crate::core::theme::ThemeMode::Dark
    } else {
        crate::core::theme::ThemeMode::Light
    }
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
            // `goyda-cli`'s `r` (quick reload) sends this - see
            // `goyda-cli/src/targets/windows`'s reload trigger. `WM_COPYDATA`
            // is the standard cross-*process* messaging mechanism (unlike
            // plain `WPARAM`/`LPARAM`, which are just integers with no
            // meaning across a process boundary, `WM_COPYDATA`'s payload is
            // kernel-marshaled into this process automatically), carrying
            // the freshly rebuilt consumer `cdylib`'s path as UTF-8 bytes.
            // Loading it and swapping to it in place - no new process, no
            // window/message-loop restart - is the actual hot-reload: the
            // window, its `HWND` tree, and this whole process's memory
            // (including anything the still-running app has stashed outside
            // a page's own `Signal`s) are completely undisturbed by this.
            WM_COPYDATA => {
                let cds = &*(lparam as *const COPYDATASTRUCT);
                if !cds.lpData.is_null() && cds.cbData > 0 {
                    let bytes = std::slice::from_raw_parts(cds.lpData as *const u8, cds.cbData as usize);
                    if let Ok(path) = std::str::from_utf8(bytes) {
                        hot_swap_dylib(path);
                    }
                }
                1
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

fn mount(hinstance: HINSTANCE, root: HWND, page: &Page, fit_window: bool) {
    // Wrapped in a scroll view so a page taller than the window scrolls
    // instead of silently clipping at the bottom edge - `AlignItems`
    // defaults to `Stretch` (see `state::ControlState::default`), so the
    // wrapped page still fills the full window width same as it did
    // unwrapped, only trading a `MATCH_PARENT`-equivalent height for one
    // that can grow past the viewport, same reasoning as android's own
    // `wrap_scrollable` in `android::bootstrap`.
    let component = Component::scroll_view(LayoutDirection::Vertical, 0, vec![(page.factory)()]);
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

fn remount(page: &Page) {
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

/// Navigates the app to the `#[page(...)]` registered for `path`.
pub fn navigate(path: &str) {
    let Some(page) = find_page(path) else {
        #[cfg(debug_assertions)]
        eprintln!("goyda(windows): navigate(\"{path}\") - no #[page(...)] registered for that route");
        return;
    };

    remount(page);
    CURRENT_PATH.with(|p| *p.borrow_mut() = path.to_string());
}

/// Rebuilds and redisplays the currently mounted page in place, without
/// changing the route.
pub fn rerender() {
    let path = CURRENT_PATH.with(|p| p.borrow().clone());
    let Some(page) = find_page(&path) else { return };
    remount(page);
}

fn hot_swap_dylib(path: &str) {
    let wide_path = wide(path);
    let handle = unsafe { LoadLibraryW(wide_path.as_ptr()) };
    if handle.is_null() {
        return;
    }

    let current_path = CURRENT_PATH.with(|p| p.borrow().clone());
    let Some(page) = find_page(&current_path) else { return };
    remount(page);
}

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

/// Runs the app: creates the window, mounts the initial `#[page("/")]`,
/// and pumps the message loop until the window closes.
///
/// `initial_dylib`, when given, is loaded first so its `#[page(...)]`s are
/// registered before the initial page is looked up.
pub fn run(initial_dylib: Option<&std::path::Path>) {
    install_panic_hook();

    if let Some(path) = initial_dylib {
        let wide_path = wide(&path.to_string_lossy());
        unsafe {
            LoadLibraryW(wide_path.as_ptr());
        }
    }

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

    crate::core::theme::init_theme_mode(detect_theme_mode());

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
