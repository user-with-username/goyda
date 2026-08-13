use std::cell::RefCell;
use std::collections::HashSet;

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use crate::components::{Asset, Axis, Color, Edge, LayoutDirection, StyleProperty, StyleValue};
use crate::core::events::Update;
use crate::core::{Backend, BackendUpdater};

fn document() -> Document {
    web_sys::window()
        .expect("goyda(web): no global `window` - is this running in a browser?")
        .document()
        .expect("goyda(web): window has no `document`")
}

fn css_color(color: Color) -> String {
    goyda_utils::color::css_rgba(color)
}

fn resolve_length(value: &StyleValue) -> Option<f32> {
    let v = goyda_utils::style::resolve_length(value);
    if v.is_none() {
        if let StyleValue::Spacing(scale) = value {
            #[cfg(debug_assertions)]
            web_sys::console::warn_1(
                &format!("goyda(web): spacing scale index {scale} out of range").into(),
            );
        }
    }
    v
}

thread_local! {
    static ASSET_BLOB_URLS: RefCell<std::collections::HashMap<*const u8, String>> = RefCell::new(std::collections::HashMap::new());
}

/// A URL the browser can load `asset` from: for an embedded asset, a
/// `blob:` URL over its bytes (cached per asset, created once); otherwise
/// the path resolved against the web build's `assets/` directory, same as
/// it always has been.
fn asset_url(asset: &Asset) -> String {
    let Some(bytes) = asset.bytes() else {
        return format!("assets/{}", asset.path());
    };

    let key = bytes.as_ptr();
    if let Some(url) = ASSET_BLOB_URLS.with(|cache| cache.borrow().get(&key).cloned()) {
        return url;
    }

    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::new();
    parts.push(&array);

    let bag = web_sys::BlobPropertyBag::new();
    if let Some(mime) = goyda_utils::asset::mime_type(asset) {
        bag.set_type(mime);
    }
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &bag)
        .expect("goyda(web): failed to construct Blob from embedded asset bytes");
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .expect("goyda(web): failed to create object URL for embedded asset");

    ASSET_BLOB_URLS.with(|cache| cache.borrow_mut().insert(key, url.clone()));
    url
}

thread_local! {
    static INJECTED_FONT_FACES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static SWITCH_STYLE_INJECTED: RefCell<bool> = RefCell::new(false);
}

/// Injects the `.goyda-switch`/`.goyda-switch-track` rules `create_switch`
/// relies on into `<head>` (once per page - subsequent calls are no-ops).
fn ensure_switch_style_injected() {
    let already_injected = SWITCH_STYLE_INJECTED.with(|f| *f.borrow());
    if already_injected {
        return;
    }

    const RULE: &str = "\
        .goyda-switch{position:relative;display:inline-flex;width:40px;height:24px;flex-shrink:0;}\
        .goyda-switch input{position:absolute;inset:0;width:100%;height:100%;margin:0;opacity:0;cursor:pointer;}\
        .goyda-switch .goyda-switch-track{position:absolute;inset:0;background:#c8c8c8;border-radius:12px;transition:background .15s ease;pointer-events:none;}\
        .goyda-switch .goyda-switch-track::before{content:\"\";position:absolute;left:3px;top:3px;width:18px;height:18px;background:#fff;border-radius:50%;transition:transform .15s ease;box-shadow:0 1px 2px rgba(0,0,0,.3);}\
        .goyda-switch input:checked+.goyda-switch-track{background:#2196f3;}\
        .goyda-switch input:checked+.goyda-switch-track::before{transform:translateX(16px);}\
    ";

    if let Some(head) = document().head() {
        if let Ok(style_element) = document().create_element("style") {
            style_element.set_text_content(Some(RULE));
            if head.append_child(&style_element).is_ok() {
                SWITCH_STYLE_INJECTED.with(|f| *f.borrow_mut() = true);
            }
        }
    }
}

/// A CSS-safe, stable `font-family` name derived from the asset path.
fn font_family_name(asset: &Asset) -> String {
    let sanitized: String = asset
        .path()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("goyda-font-{sanitized}")
}

/// Injects an `@font-face` rule for `asset` into `<head>` (once per unique
/// asset path - subsequent calls are no-ops) and returns the `font-family`
/// name it registered under.
fn ensure_font_face_injected(asset: &Asset) -> String {
    let family = font_family_name(asset);

    let already_injected = INJECTED_FONT_FACES.with(|set| set.borrow().contains(&family));
    if already_injected {
        return family;
    }

    let format_decl = goyda_utils::asset::font_format_hint(asset)
        .map(|hint| format!(" format(\"{hint}\")"))
        .unwrap_or_default();
    let rule = format!(
        "@font-face {{ font-family: \"{family}\"; src: url(\"{}\"){format_decl}; }}",
        asset_url(asset)
    );

    if let Some(head) = document().head() {
        if let Ok(style_element) = document().create_element("style") {
            style_element.set_text_content(Some(&rule));
            if head.append_child(&style_element).is_ok() {
                INJECTED_FONT_FACES.with(|set| {
                    set.borrow_mut().insert(family.clone());
                });
            }
        }
    }

    family
}

fn set_style(element: &Element, prop: &str, value: &str) {
    if let Some(html_element) = element.dyn_ref::<HtmlElement>() {
        let _ = html_element.style().set_property(prop, value);
    }
}

fn apply_style_to_element(element: &Element, StyleProperty(axis, value): &StyleProperty) {
    match axis {
        Axis::TextColor => {
            if let StyleValue::Color(c) = value {
                set_style(element, "color", &css_color(*c));
            }
        }
        Axis::BackgroundColor => {
            if let StyleValue::Color(c) = value {
                set_style(element, "background-color", &css_color(*c));
            }
        }
        Axis::BorderColor => {
            if let StyleValue::Color(c) = value {
                set_style(element, "border-color", &css_color(*c));
                set_style(element, "border-style", "solid");
            }
        }
        Axis::BorderWidth => {
            if let Some(v) = resolve_length(value) {
                set_style(element, "border-width", &format!("{v}px"));
                set_style(element, "border-style", "solid");
            }
        }
        Axis::BorderRadius => {
            if let Some(v) = resolve_length(value) {
                set_style(element, "border-radius", &format!("{v}px"));
            }
        }
        Axis::Shadow => {
            if let Some(v) = resolve_length(value) {
                set_style(element, "box-shadow", &format!("0 {v}px {v}px rgba(0, 0, 0, 0.25)"));
            }
        }
        Axis::Opacity => {
            if let StyleValue::Number(alpha) = value {
                set_style(element, "opacity", &alpha.to_string());
            }
        }
        Axis::FontSize => {
            if let Some(v) = resolve_length(value) {
                set_style(element, "font-size", &format!("{v}px"));
            }
        }
        Axis::FontFamily => {
            if let StyleValue::Asset(asset) = value {
                let family = ensure_font_face_injected(asset);
                set_style(element, "font-family", &format!("\"{family}\", sans-serif"));
            }
        }
        Axis::Padding(edge) => {
            if let Some(v) = resolve_length(value) {
                apply_edge(element, "padding", *edge, v);
            }
        }
        Axis::Margin(edge) => {
            if let Some(v) = resolve_length(value) {
                apply_edge(element, "margin", *edge, v);
            }
        }
    }
}

fn apply_edge(element: &Element, prop: &str, edge: Edge, v: f32) {
    let px = format!("{v}px");
    match edge {
        Edge::All => set_style(element, prop, &px),
        Edge::Horizontal => {
            set_style(element, &format!("{prop}-left"), &px);
            set_style(element, &format!("{prop}-right"), &px);
        }
        Edge::Vertical => {
            set_style(element, &format!("{prop}-top"), &px);
            set_style(element, &format!("{prop}-bottom"), &px);
        }
        Edge::Top => set_style(element, &format!("{prop}-top"), &px),
        Edge::Right => set_style(element, &format!("{prop}-right"), &px),
        Edge::Bottom => set_style(element, &format!("{prop}-bottom"), &px),
        Edge::Left => set_style(element, &format!("{prop}-left"), &px),
    }
}

#[derive(Clone)]
pub struct WebView {
    pub element: Element,
}

#[derive(Clone)]
pub struct WebUpdater;

impl BackendUpdater for WebUpdater {
    type PlatformView = WebView;

    fn apply_update(&mut self, view: &Self::PlatformView, update: Update) {
        match update {
            Update::SetText(content) => {
                view.element.set_text_content(Some(&content));
            }
            Update::SetProgress(value) => {
                if let Some(input) = view.element.dyn_ref::<web_sys::HtmlInputElement>() {
                    input.set_value(&value.clamp(0.0, 1.0).to_string());
                }
            }
        }
    }
}

#[derive(Default)]
pub struct WebBackend;

impl WebBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Backend for WebBackend {
    type PlatformView = WebView;
    type Updater = WebUpdater;

    fn clone_updater(&self) -> Self::Updater {
        WebUpdater
    }

    fn create_text(&mut self, content: &str) -> Self::PlatformView {
        let element = document()
            .create_element("span")
            .expect("goyda(web): failed to create <span>");
        element.set_text_content(Some(content));
        WebView { element }
    }

    fn create_button(&mut self, text: &str) -> Self::PlatformView {
        let element = document()
            .create_element("button")
            .expect("goyda(web): failed to create <button>");
        element.set_text_content(Some(text));
        set_style(&element, "cursor", "pointer");
        set_style(&element, "font", "inherit");
        WebView { element }
    }

    fn create_image(&mut self, asset: &Asset) -> Self::PlatformView {
        let element = document()
            .create_element("img")
            .expect("goyda(web): failed to create <img>");
        let _ = element.set_attribute("src", &asset_url(asset));

        // Opt out of the parent stack's default `align-items: stretch` -
        // without this an image with no explicit size would be stretched to
        // fill the stack's cross-axis instead of showing at its own size.
        set_style(&element, "align-self", "flex-start");
        set_style(&element, "max-width", "100%");
        set_style(&element, "height", "auto");

        WebView { element }
    }

    fn create_text_input(&mut self, placeholder: &str, initial_text: &str) -> Self::PlatformView {
        let element = document()
            .create_element("input")
            .expect("goyda(web): failed to create <input>");
        let _ = element.set_attribute("type", "text");
        let _ = element.set_attribute("placeholder", placeholder);
        if !initial_text.is_empty() {
            let _ = element.set_attribute("value", initial_text);
        }
        set_style(&element, "font", "inherit");
        set_style(&element, "box-sizing", "border-box");
        WebView { element }
    }

    fn create_checkbox(&mut self, label: &str, checked: bool) -> Self::PlatformView {
        let wrapper = document()
            .create_element("label")
            .expect("goyda(web): failed to create <label>");
        set_style(&wrapper, "display", "inline-flex");
        set_style(&wrapper, "align-items", "center");
        set_style(&wrapper, "gap", "6px");
        set_style(&wrapper, "cursor", "pointer");

        let input = document()
            .create_element("input")
            .expect("goyda(web): failed to create <input>");
        let _ = input.set_attribute("type", "checkbox");
        if checked {
            let _ = input.set_attribute("checked", "checked");
        }
        let _ = wrapper.append_child(&input);

        if !label.is_empty() {
            let text = document().create_element("span").expect("goyda(web): failed to create <span>");
            text.set_text_content(Some(label));
            let _ = wrapper.append_child(&text);
        }

        WebView { element: wrapper }
    }

    fn create_switch(&mut self, checked: bool) -> Self::PlatformView {
        // No native `<switch>` element exists, so this reuses a real
        // `input[type=checkbox]` for state/click-toggle/keyboard behavior
        // (same element the `checked_change` listener's `change` handler
        // already expects, see `crate::listeners` - `change` bubbles, so
        // attaching to this wrapping `<label>` still works) - visually
        // hidden and paired with a sibling `<span>` track/thumb, since an
        // `appearance: none` checkbox alone has no `:checked` visual state
        // of its own to show which way it's toggled.
        ensure_switch_style_injected();

        let wrapper = document().create_element("label").expect("goyda(web): failed to create <label>");
        let _ = wrapper.set_attribute("class", "goyda-switch");

        let input = document().create_element("input").expect("goyda(web): failed to create <input>");
        let _ = input.set_attribute("type", "checkbox");
        if checked {
            let _ = input.set_attribute("checked", "checked");
        }
        let _ = wrapper.append_child(&input);

        let track = document().create_element("span").expect("goyda(web): failed to create <span>");
        let _ = track.set_attribute("class", "goyda-switch-track");
        let _ = wrapper.append_child(&track);

        WebView { element: wrapper }
    }

    fn create_progress(&mut self, value: f32) -> Self::PlatformView {
        // A scrubber, not just a read-only indicator - `<input type=range>`
        // gives click/drag-to-seek for free (report changes to the app with
        // `.on_value_changed(...)`, wired to this element's `input` event by
        // the `seek` listener - see `crate::listeners`), which a plain
        // `<progress>` doesn't support.
        let element = document()
            .create_element("input")
            .expect("goyda(web): failed to create <input>");
        let _ = element.set_attribute("type", "range");
        let _ = element.set_attribute("min", "0");
        let _ = element.set_attribute("max", "1");
        let _ = element.set_attribute("step", "0.001");
        let _ = element.set_attribute("value", &value.clamp(0.0, 1.0).to_string());
        set_style(&element, "width", "100%");
        WebView { element }
    }

    fn create_spacer(&mut self, size: i32) -> Self::PlatformView {
        let element = document()
            .create_element("div")
            .expect("goyda(web): failed to create <div>");
        set_style(&element, "width", &format!("{size}px"));
        set_style(&element, "height", &format!("{size}px"));
        set_style(&element, "flex-shrink", "0");
        WebView { element }
    }

    fn create_divider(&mut self) -> Self::PlatformView {
        let element = document()
            .create_element("hr")
            .expect("goyda(web): failed to create <hr>");
        set_style(&element, "width", "100%");
        set_style(&element, "border", "none");
        set_style(&element, "border-top", "1px solid #c8c8c8");
        set_style(&element, "margin", "0");
        set_style(&element, "flex-shrink", "0");
        WebView { element }
    }

    fn create_stack(
        &mut self,
        direction: LayoutDirection,
        spacing: i32,
        children: Vec<Self::PlatformView>,
    ) -> Self::PlatformView {
        let element = document()
            .create_element("div")
            .expect("goyda(web): failed to create <div>");

        set_style(&element, "display", "flex");
        set_style(
            &element,
            "flex-direction",
            match direction {
                LayoutDirection::Horizontal => "row",
                LayoutDirection::Vertical => "column",
            },
        );
        if spacing > 0 {
            set_style(&element, "gap", &format!("{spacing}px"));
        }

        for child in children {
            let _ = element.append_child(&child.element);
        }

        WebView { element }
    }

    fn apply_style(&mut self, view: &Self::PlatformView, style: StyleProperty) {
        apply_style_to_element(&view.element, &style);
    }
}
