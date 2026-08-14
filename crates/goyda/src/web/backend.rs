use std::cell::RefCell;
use std::collections::HashSet;

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use crate::components::{
    Align, Asset, Axis, Color, Edge, LayoutDirection, StyleProperty, StyleValue,
};
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

fn font_family_name(asset: &Asset) -> String {
    let sanitized: String = asset
        .path()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("goyda-font-{sanitized}")
}

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
                // Stashed in a data attribute so a later `.shadow_color(...)`
                // (see `Axis::ShadowColor` below) can re-issue `box-shadow`
                // at the same size with a different color, without needing
                // to parse the shorthand back out of the inline style.
                let _ = element.set_attribute("data-shadow-px", &v.to_string());
                set_style(
                    element,
                    "box-shadow",
                    &format!("0 {v}px {v}px rgba(0, 0, 0, 0.25)"),
                );
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
        Axis::Width => {
            if let Some(v) = resolve_length(value) {
                set_style(element, "width", &format!("{v}px"));
                set_style(element, "flex-shrink", "0");
            }
        }
        Axis::Height => {
            if let Some(v) = resolve_length(value) {
                set_style(element, "height", &format!("{v}px"));
                set_style(element, "flex-shrink", "0");
            }
        }
        Axis::FontWeight => {
            if let StyleValue::Bool(bold) = value {
                set_style(
                    element,
                    "font-weight",
                    if *bold { "bold" } else { "normal" },
                );
            }
        }
        Axis::FontStyle => {
            if let StyleValue::Bool(italic) = value {
                set_style(
                    element,
                    "font-style",
                    if *italic { "italic" } else { "normal" },
                );
            }
        }
        Axis::TextAlign => {
            if let StyleValue::Align(a) = value {
                set_style(element, "text-align", css_text_align(*a));
            }
        }
        Axis::AlignItems => {
            if let StyleValue::Align(a) = value {
                set_style(element, "align-items", css_align_items(*a));
            }
        }
        Axis::JustifyContent => {
            if let StyleValue::Align(a) = value {
                set_style(element, "justify-content", css_justify_content(*a));
            }
        }
        Axis::LineHeight => {
            if let Some(v) = resolve_length(value) {
                set_style(element, "line-height", &format!("{v}px"));
            }
        }
        Axis::LetterSpacing => {
            if let Some(v) = resolve_length(value) {
                set_style(element, "letter-spacing", &format!("{v}px"));
            }
        }
        Axis::Underline => {
            if let StyleValue::Bool(v) = value {
                set_style(
                    element,
                    "text-decoration",
                    if *v { "underline" } else { "none" },
                );
            }
        }
        Axis::Strikethrough => {
            if let StyleValue::Bool(v) = value {
                set_style(
                    element,
                    "text-decoration",
                    if *v { "line-through" } else { "none" },
                );
            }
        }
        Axis::TextOverflowEllipsis => {
            if let StyleValue::Bool(true) = value {
                set_style(element, "overflow", "hidden");
                set_style(element, "white-space", "nowrap");
                set_style(element, "text-overflow", "ellipsis");
            }
        }
        Axis::Clip => {
            if let StyleValue::Bool(v) = value {
                set_style(element, "overflow", if *v { "hidden" } else { "visible" });
            }
        }
        Axis::ShadowColor => {
            // `.shadow(px)` must run first (declaration order = application
            // order for a `Styled` chain) so `data-shadow-px` is already
            // stashed - see `Axis::Shadow` above.
            if let StyleValue::Color(c) = value {
                let shadow_px: f32 = element
                    .get_attribute("data-shadow-px")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(4.0);
                set_style(
                    element,
                    "box-shadow",
                    &format!("0 {shadow_px}px {shadow_px}px {}", css_color(*c)),
                );
            }
        }
        Axis::OffsetX => {
            if let Some(v) = resolve_length(value) {
                set_style(element, "left", &format!("{v}px"));
            }
        }
        Axis::OffsetY => {
            if let Some(v) = resolve_length(value) {
                set_style(element, "top", &format!("{v}px"));
            }
        }
        Axis::ZIndex => {
            if let StyleValue::Number(z) = value {
                set_style(element, "z-index", &(*z as i32).to_string());
            }
        }
    }
}

fn css_text_align(align: Align) -> &'static str {
    match align {
        Align::Start => "left",
        Align::Center => "center",
        Align::End => "right",
        Align::Stretch | Align::SpaceBetween => "left",
    }
}

fn css_align_items(align: Align) -> &'static str {
    match align {
        Align::Start => "flex-start",
        Align::Center => "center",
        Align::End => "flex-end",
        Align::Stretch => "stretch",
        Align::SpaceBetween => "stretch",
    }
}

fn css_justify_content(align: Align) -> &'static str {
    match align {
        Align::Start => "flex-start",
        Align::Center => "center",
        Align::End => "flex-end",
        Align::Stretch => "flex-start",
        Align::SpaceBetween => "space-between",
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

/// A handle to a mounted DOM element.
#[derive(Clone)]
pub struct WebView {
    pub element: Element,
}

/// Applies reactive updates to DOM elements.
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

/// The web rendering backend, mounting components as DOM elements.
#[derive(Default)]
pub struct WebBackend;

impl WebBackend {
    /// Creates a new web backend.
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

    fn create_textarea(&mut self, placeholder: &str, initial_text: &str) -> Self::PlatformView {
        let element = document()
            .create_element("textarea")
            .expect("goyda(web): failed to create <textarea>");
        let _ = element.set_attribute("placeholder", placeholder);
        let _ = element.set_attribute("rows", "4");
        element.set_text_content(Some(initial_text));
        set_style(&element, "font", "inherit");
        set_style(&element, "box-sizing", "border-box");
        set_style(&element, "width", "100%");
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
            let text = document()
                .create_element("span")
                .expect("goyda(web): failed to create <span>");
            text.set_text_content(Some(label));
            let _ = wrapper.append_child(&text);
        }

        WebView { element: wrapper }
    }

    fn create_radio_button(
        &mut self,
        group: &str,
        label: &str,
        selected: bool,
    ) -> Self::PlatformView {
        // `name="{group}"` gives mutual exclusion for free - the browser
        // itself deselects every other `input[type=radio]` sharing that
        // name the moment this one gets selected, no extra JS needed.
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
        let _ = input.set_attribute("type", "radio");
        let _ = input.set_attribute("name", group);
        if selected {
            let _ = input.set_attribute("checked", "checked");
        }
        let _ = wrapper.append_child(&input);

        if !label.is_empty() {
            let text = document()
                .create_element("span")
                .expect("goyda(web): failed to create <span>");
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

        let wrapper = document()
            .create_element("label")
            .expect("goyda(web): failed to create <label>");
        let _ = wrapper.set_attribute("class", "goyda-switch");

        let input = document()
            .create_element("input")
            .expect("goyda(web): failed to create <input>");
        let _ = input.set_attribute("type", "checkbox");
        if checked {
            let _ = input.set_attribute("checked", "checked");
        }
        let _ = wrapper.append_child(&input);

        let track = document()
            .create_element("span")
            .expect("goyda(web): failed to create <span>");
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

    fn create_scroll_view(
        &mut self,
        direction: LayoutDirection,
        spacing: i32,
        children: Vec<Self::PlatformView>,
    ) -> Self::PlatformView {
        let view = self.create_stack(direction, spacing, children);
        match direction {
            LayoutDirection::Vertical => set_style(&view.element, "overflow-y", "auto"),
            LayoutDirection::Horizontal => set_style(&view.element, "overflow-x", "auto"),
        }
        view
    }

    fn create_overlay(&mut self, children: Vec<Self::PlatformView>) -> Self::PlatformView {
        let element = document()
            .create_element("div")
            .expect("goyda(web): failed to create <div>");
        set_style(&element, "position", "relative");

        for child in &children {
            // `left`/`top`/`z-index` (see `Axis::OffsetX`/`OffsetY`/`ZIndex`)
            // already applied to `child.element` by the time it got here -
            // this just switches it out of normal flow so those actually
            // do something.
            set_style(&child.element, "position", "absolute");
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
