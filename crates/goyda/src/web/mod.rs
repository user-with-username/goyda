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

/// Entry point wasm-bindgen invokes automatically once the module is
/// instantiated in the browser - no glue code is required in consumer
/// crates, matching how android pages need no manual bootstrap either.
#[wasm_bindgen(start)]
pub fn __goyda_web_start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

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
    on_pop_state.forget();

    Ok(())
}
