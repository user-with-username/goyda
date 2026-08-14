use crate::components::Component;

pub struct RadioButton {
    pub group: String,
    pub label: String,
    pub selected: bool,
}

impl RadioButton {
    /// A labeled radio button - selecting it deselects every other
    /// `RadioButton` created with the same `group` string, so a group of
    /// options just needs to share one. Listen for selection with
    /// `.on_checked_change(...)`, same as [`crate::components::Checkbox`]
    /// (fires with `true` once, when this button becomes selected - never
    /// `false`, since a radio button only ever deselects because a sibling
    /// got selected instead).
    pub fn new(group: impl Into<String>, label: impl Into<String>, selected: bool) -> Component {
        Component::RadioButton(Self {
            group: group.into(),
            label: label.into(),
            selected,
        })
    }
}
