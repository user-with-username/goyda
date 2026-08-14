pub mod backend;

pub use backend::WebBackend;

use std::cell::RefCell;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use crate::{Page, find_page};

fn current_path() -> String {
    web_sys::window()
        .and_then(|w| w.location().pathname().ok())
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| "/".to_string())
}

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
    static MOUNT: RefCell<Option<(WebBackend, web_sys::Element)>> = RefCell::new(None);
    static POPSTATE_CLOSURE: RefCell<Option<Closure<dyn Fn()>>> = RefCell::new(None);
}

fn render_page(page: &Page) {
    MOUNT.with(|cell| {
        let mut slot = cell.borrow_mut();
        let (backend, container) = slot.get_or_insert_with(|| {
            (
                WebBackend::new(),
                mount_root().expect("goyda(web): failed to find a mount root"),
            )
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

/// Navigates the app to the `#[page(...)]` registered for `path`, updating
/// the browser's URL.
pub fn navigate(path: &str) {
    let Some(page) = find_page(path) else {
        #[cfg(debug_assertions)]
        web_sys::console::warn_1(
            &format!(
                "goyda(web): navigate(\"{path}\") - no #[page(...)] registered for that route"
            )
            .into(),
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

fn handle_pop_state() {
    if let Some(page) = find_page(&current_path()) {
        render_page(page);
    }
}

/// Rebuilds and redisplays the currently mounted page in place, without
/// changing the route.
pub fn rerender() {
    if let Some(page) = find_page(&current_path()) {
        render_page(page);
    }
}

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

/// Starts the app: mounts the initial `#[page(...)]` for the current URL
/// and sets up back/forward navigation. Called once from JS after the wasm
/// module loads.
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

    let window =
        web_sys::window().ok_or_else(|| JsValue::from_str("goyda(web): no global `window`"))?;
    let on_pop_state = Closure::<dyn Fn()>::new(handle_pop_state);
    window
        .add_event_listener_with_callback("popstate", on_pop_state.as_ref().unchecked_ref())
        .map_err(|_| JsValue::from_str("goyda(web): failed to attach popstate listener"))?;
    POPSTATE_CLOSURE.with(|cell| *cell.borrow_mut() = Some(on_pop_state));

    Ok(())
}

/// Snapshots the app's reactive state as JSON, for restoring with
/// [`goyda_install_state`] after a hot reload.
#[wasm_bindgen]
pub fn goyda_dump_state() -> String {
    crate::reactive::dump_state()
}

/// Restores reactive state previously captured with [`goyda_dump_state`].
/// Call before [`goyda_start`].
#[wasm_bindgen]
pub fn goyda_install_state(json: String) {
    crate::reactive::install_state(&json);
}

/// Detaches the app's browser event listeners. Call before discarding the
/// wasm module, e.g. ahead of a hot reload.
#[wasm_bindgen]
pub fn goyda_teardown() {
    let Some(window) = web_sys::window() else {
        return;
    };
    POPSTATE_CLOSURE.with(|cell| {
        if let Some(closure) = cell.borrow_mut().take() {
            let _ = window
                .remove_event_listener_with_callback("popstate", closure.as_ref().unchecked_ref());
        }
    });
}
