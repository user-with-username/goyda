pub mod backend;

pub use backend::WebBackend;

use std::cell::RefCell;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::{find_page, Page};

fn current_path() -> String {
    web_sys::window()
        .and_then(|w| w.location().pathname().ok())
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| "/".to_string())
}

/// The DOM node goyda mounts the app's root component into. If the page's
/// `index.html` has `<div id="app">`, goyda renders there; otherwise it
/// falls back to `<body>` directly.
fn mount_root() -> Result<web_sys::Element, JsValue> {
    let document = web_sys::window()
        .ok_or_else(|| JsValue::from_str("goyda(web): no global `window`"))?
        .document()
        .ok_or_else(|| JsValue::from_str("goyda(web): window has no `document`"))?;

    if let Some(el) = document.get_element_by_id("app") {
        return Ok(el);
    }

    document
        .body()
        .ok_or_else(|| JsValue::from_str("goyda(web): document has no <body> and no #app element"))?
        .dyn_into::<web_sys::Element>()
        .map_err(|_| JsValue::from_str("goyda(web): failed to use <body> as the mount root"))
}

thread_local! {
    /// The `WebBackend` and root DOM node stay fixed for the app's lifetime -
    /// only the mounted component tree changes on navigation.
    static MOUNT: RefCell<Option<(WebBackend, web_sys::Element)>> = RefCell::new(None);
    /// Kept (rather than `.forget()`-ten) so [`goyda_teardown`] can actually
    /// remove it from `window` before a hot reload discards this module -
    /// a `.forget()`-ten closure stays registered forever, and calling into
    /// it after this module's instance is gone (the next `popstate`, e.g.
    /// the user pressing the browser's back button) would reach into freed
    /// wasm memory.
    static POPSTATE_CLOSURE: RefCell<Option<Closure<dyn Fn()>>> = RefCell::new(None);
}

/// Renders `page` and mounts it into the app's root, replacing whatever was
/// there before. Used for both the initial load and every [`navigate`] call.
fn render_page(page: &Page) {
    MOUNT.with(|cell| {
        let mut slot = cell.borrow_mut();
        let (backend, container) = slot.get_or_insert_with(|| {
            (WebBackend::new(), mount_root().expect("goyda(web): failed to find a mount root"))
        });

        let component = (page.factory)();
        let view = component.render(backend);

        container.set_text_content(None);
        let _ = container.append_child(&view.element);

        // Leak the just-rendered tree instead of storing it: its `Signal`
        // subscriptions must outlive this function call to keep driving
        // `Update`s, and a page swap is expected to be a rare, deliberate
        // navigation - not something that needs to reclaim the previous
        // page's memory immediately.
        std::mem::forget(component);
    });
}

/// Switches the mounted app to whichever `#[page(...)]` is registered for
/// `path` (see [`crate::find_page`]), and updates the browser's URL bar via
/// `history.pushState` so the address bar, reload, and share links all stay
/// in sync with what's on screen.
pub fn navigate(path: &str) {
    let Some(page) = find_page(path) else {
        #[cfg(debug_assertions)]
        web_sys::console::warn_1(
            &format!("goyda(web): navigate(\"{path}\") - no #[page(...)] registered for that route").into(),
        );
        return;
    };

    if let Some(window) = web_sys::window() {
        if let Ok(history) = window.history() {
            let _ = history.push_state_with_url(&JsValue::NULL, "", Some(path));
        }
    }

    render_page(page);
}

/// Re-renders whatever `#[page(...)]` matches the current URL - wired to the
/// `popstate` event so the browser's back/forward buttons work like real
/// navigation instead of leaving the on-screen page out of sync.
fn handle_pop_state() {
    if let Some(page) = find_page(&current_path()) {
        render_page(page);
    }
}

/// Re-renders whatever `#[page(...)]` matches the current URL, in place -
/// same route, no `pushState` call. Used by
/// [`crate::core::theme::set_theme_mode`] so a runtime `theme!` switch
/// shows up immediately.
pub fn rerender() {
    if let Some(page) = find_page(&current_path()) {
        render_page(page);
    }
}

/// Reads the `(prefers-color-scheme: dark)` media query to seed the initial
/// [`crate::core::theme::ThemeMode`] from whatever the browser/OS is
/// actually set to - see `crate::core::theme`'s doc comment for how this
/// plugs into `theme!`.
fn detect_theme_mode() -> crate::core::theme::ThemeMode {
    let prefers_dark = web_sys::window()
        .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok().flatten())
        .map(|mql| mql.matches())
        .unwrap_or(false);

    if prefers_dark {
        crate::core::theme::ThemeMode::Dark
    } else {
        crate::core::theme::ThemeMode::Light
    }
}

/// The app's real entry point - called explicitly from `index.html`'s
/// bootstrap script (see `goyda-cli`'s web target) right after the wasm
/// module finishes instantiating, rather than via wasm-bindgen's
/// `#[wasm_bindgen(start)]` auto-invoke: a hot reload needs [`goyda_install_state`]
/// to run *between* instantiation and the first page mount (so
/// `Signal::new_keyed` sees restored values from the very first render),
/// and `#[wasm_bindgen(start)]` gives JS no chance to call anything in that
/// gap - it fires synchronously as part of instantiation itself. An
/// ordinary (non-reload) first load just calls this with nothing to
/// install, which is exactly today's behavior.
#[wasm_bindgen]
pub fn goyda_start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    crate::core::theme::init_theme_mode(detect_theme_mode());

    let path = current_path();
    let page = find_page(&path).ok_or_else(|| {
        JsValue::from_str(&format!(
            "goyda(web): no #[page(...)] registered for route '{path}' (and no '/' fallback)"
        ))
    })?;

    render_page(page);

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("goyda(web): no global `window`"))?;
    let on_pop_state = Closure::<dyn Fn()>::new(handle_pop_state);
    window
        .add_event_listener_with_callback("popstate", on_pop_state.as_ref().unchecked_ref())
        .map_err(|_| JsValue::from_str("goyda(web): failed to attach popstate listener"))?;
    POPSTATE_CLOSURE.with(|cell| *cell.borrow_mut() = Some(on_pop_state));

    Ok(())
}

/// JS-exposed wrapper around [`crate::reactive::dump_state`] - see that
/// function's doc comment. Called on the *old* module right before a hot
/// reload discards it.
#[wasm_bindgen]
pub fn goyda_dump_state() -> String {
    crate::reactive::dump_state()
}

/// JS-exposed wrapper around [`crate::reactive::install_state`] - called on
/// the *new* module, before [`goyda_start`], with whatever
/// [`goyda_dump_state`] returned from the module it's replacing.
#[wasm_bindgen]
pub fn goyda_install_state(json: String) {
    crate::reactive::install_state(&json);
}

/// Removes the `popstate` listener [`goyda_start`] registered - called on
/// the *old* module right before a hot reload discards it, so the browser
/// never calls back into this (about to be freed) module's memory. Only
/// `popstate` needs this: every other listener in this backend is attached
/// to an individual rendered DOM node (see `web/backend.rs`), which stops
/// receiving events the moment [`render_page`] replaces it - `window`
/// itself isn't torn down by a hot reload, so anything attached directly to
/// it needs explicit cleanup.
#[wasm_bindgen]
pub fn goyda_teardown() {
    let Some(window) = web_sys::window() else { return };
    POPSTATE_CLOSURE.with(|cell| {
        if let Some(closure) = cell.borrow_mut().take() {
            let _ = window.remove_event_listener_with_callback("popstate", closure.as_ref().unchecked_ref());
        }
    });
}
