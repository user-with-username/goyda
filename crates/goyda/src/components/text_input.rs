use crate::components::Component;

pub struct TextInput {
    pub placeholder: String,
    pub initial_text: String,
}

impl TextInput {
    /// A single-line editable text field. Listen for edits with
    /// `.on_text_watcher(...)` (see [`crate::listeners`]) - the platform
    /// gives back the field's full current text on every keystroke.
    pub fn new(placeholder: impl Into<String>) -> Component {
        Component::TextInput(Self {
            placeholder: placeholder.into(),
            initial_text: String::new(),
        })
    }

    /// Same as [`TextInput::new`], pre-filled with `initial_text`.
    pub fn with_text(placeholder: impl Into<String>, initial_text: impl Into<String>) -> Component {
        Component::TextInput(Self {
            placeholder: placeholder.into(),
            initial_text: initial_text.into(),
        })
    }
}
