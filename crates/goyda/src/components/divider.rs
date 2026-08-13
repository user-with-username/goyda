use crate::components::Component;

pub struct Divider;

impl Divider {
    /// A thin line spanning the cross axis of the enclosing
    /// [`Stack`](crate::components::Stack) - style it with `.background(...)`
    /// (line color) and `.py`/`.px` (thickness is fixed at 1px by default,
    /// use padding on a wrapping stack to add breathing room around it).
    pub fn new() -> Component {
        Component::Divider(Self)
    }
}
