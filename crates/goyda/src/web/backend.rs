use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use crate::components::style::SPACING;
use crate::components::{Axis, Color, Edge, LayoutDirection, StyleProperty, StyleValue};
use crate::core::events::Update;
use crate::core::{Backend, BackendUpdater};

fn document() -> Document {
    web_sys::window()
        .expect("goyda(web): no global `window` - is this running in a browser?")
        .document()
        .expect("goyda(web): window has no `document`")
}

fn css_color(color: Color) -> String {
    let argb: u32 = match color {
        Color::PRIMARY => 0xFF6200EE,
        Color::GRAY => 0xFF888888,
        Color::GREEN => 0xFF4CAF50,
        Color::RED => 0xFFF44336,
        Color::BACKGROUND => 0xFFFFFFFF,
        Color::Custom(hex) => hex,
    };

    let a = ((argb >> 24) & 0xFF) as f32 / 255.0;
    let r = (argb >> 16) & 0xFF;
    let g = (argb >> 8) & 0xFF;
    let b = argb & 0xFF;

    format!("rgba({r}, {g}, {b}, {a:.3})")
}

fn resolve_length(value: &StyleValue) -> Option<f32> {
    match value {
        StyleValue::Length(v) => Some(*v),
        StyleValue::Spacing(scale) => {
            let v = SPACING.get(*scale).copied();
            if v.is_none() {
                #[cfg(debug_assertions)]
                web_sys::console::warn_1(
                    &format!("goyda(web): spacing scale index {scale} out of range").into(),
                );
            }
            v
        }
        _ => None,
    }
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
