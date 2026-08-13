use crate::components::Component;

pub struct Overlay {
    pub children: Vec<Component>,
}

impl Overlay {
    /// A stacking context - `position: absolute` in CSS terms. Children are
    /// positioned wherever `.offset(x, y)` puts them (top-left corner if
    /// unset) instead of flowing one after another like a
    /// [`Stack`](crate::components::Stack), and stack visually by
    /// `.z_index(n)` (creation order breaks ties). The classic use is a
    /// small badge pinned to a corner of a larger element:
    /// `Component::overlay(vec![avatar, badge.offset(24, 24)])`.
    pub fn new(children: Vec<Component>) -> Component {
        Component::Overlay(Self { children })
    }
}
