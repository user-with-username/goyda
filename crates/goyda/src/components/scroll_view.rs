use crate::components::{Component, LayoutDirection};

pub struct ScrollView {
    pub direction: LayoutDirection,
    pub spacing: i32,
    pub children: Vec<Component>,
}

impl ScrollView {
    /// A scrollable [`Stack`](crate::components::Stack) - content taller
    /// (or, for `Horizontal`, wider) than the space it's given scrolls
    /// instead of being clipped. Give it a fixed size with
    /// `.height(px)`/`.width(px)` for scrolling to actually kick in -
    /// without one it just wraps to its content's own size, the same as a
    /// plain `Stack` (nothing to scroll).
    pub fn new(direction: LayoutDirection, spacing: i32, children: Vec<Component>) -> Component {
        Component::ScrollView(Self { direction, spacing, children })
    }
}
