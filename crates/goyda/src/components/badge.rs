use crate::components::{Color, Component};

pub struct Badge;

impl Badge {
    /// A small pill-shaped status label (a [`Text`](crate::components::Text)
    /// pre-styled with a colored background, white text, and fully-rounded
    /// corners) - `color` is the badge's background (`.color(Color::RED)`
    /// for a "3 unread" count, `Color::GREEN` for "Online", ...).
    pub fn new(text: impl Into<String>, color: Color) -> Component {
        let text = text.into();
        Component::text(move || text.clone())
            .color(Color::WHITE)
            .background(color)
            .font_size(12)
            .padding(8, 2)
            .rounded(999)
    }
}
