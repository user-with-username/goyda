pub mod backend;

pub use backend::WebBackend;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::Page;

fn current_path() -> String {
    web_sys::window()
        .and_then(|w| w.location().pathname().ok())
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| "/".to_string())
}

fn find_page(path: &str) -> Option<&'static Page> {
    inventory::iter::<Page>
        .into_iter()
        .find(|p| p.route == path)
        .or_else(|| inventory::iter::<Page>.into_iter().find(|p| p.route == "/"))
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

    let component = (page.factory)();
    let mut backend = WebBackend::new();
    let view = component.render(&mut backend);

    let container = mount_root()?;
    container.set_text_content(None);
    container
        .append_child(&view.element)
        .map_err(|_| JsValue::from_str("goyda(web): failed to mount the root component"))?;

    Ok(())
}
