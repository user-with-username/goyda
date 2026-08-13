use crate::components::{Color, Component};

pub struct Link;

impl Link {
    /// Clickable text styled like a hyperlink (primary-colored) - runs
    /// `on_click` when tapped/clicked, same as `Component::button(...)
    /// .on_click(...)` but without a button's background/padding chrome.
    pub fn new(text: impl Into<String>, on_click: impl Fn() + 'static) -> Component {
        let text = text.into();
        Component::text(move || text.clone())
            .color(Color::PRIMARY)
            .on_click(on_click)
    }
}
