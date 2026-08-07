use crate::components::{Component, LayoutDirection};

pub struct Stack {
    pub direction: LayoutDirection,
    pub spacing: i32,
    pub children: Vec<Component>,
}

impl Stack {
    pub fn new(direction: LayoutDirection, spacing: i32, children: Vec<Component>) -> Component {
        Component::Stack(Self { direction, spacing, children })
    }
}
