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
