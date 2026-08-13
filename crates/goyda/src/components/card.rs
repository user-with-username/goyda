use crate::components::{Color, Component, LayoutDirection};

pub struct Card;

impl Card {
    /// A [`Stack`](crate::components::Stack) pre-styled as an elevated
    /// surface (white background, rounded corners, a soft shadow, and
    /// breathing-room padding) - the common "content grouped in a box"
    /// pattern, still a plain `Component` underneath so every style method
    /// (`.background(...)`, `.rounded(...)`, ...) can override a default.
    pub fn new(children: Vec<Component>) -> Component {
        Component::stack(LayoutDirection::Vertical, 8, children)
            .background(Color::WHITE)
            .rounded(8)
            .shadow(4)
            .padding(16, 16)
    }
}
