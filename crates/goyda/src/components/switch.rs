use crate::components::Component;

pub struct Switch {
    pub checked: bool,
}

impl Switch {
    /// An on/off toggle switch, starting `checked` or not. Listen for
    /// toggles with `.on_checked_change(...)` (see [`crate::listeners`]).
    pub fn new(checked: bool) -> Component {
        Component::Switch(Self { checked })
    }
}
