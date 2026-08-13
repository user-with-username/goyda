use crate::components::Component;

pub struct Checkbox {
    pub label: String,
    pub checked: bool,
}

impl Checkbox {
    /// A labeled checkbox, starting `checked` or not. Listen for toggles
    /// with `.on_checked_change(...)` (see [`crate::listeners`]).
    pub fn new(label: impl Into<String>, checked: bool) -> Component {
        Component::Checkbox(Self { label: label.into(), checked })
    }
}
