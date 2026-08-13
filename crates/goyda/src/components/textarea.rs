use crate::components::Component;

pub struct Textarea {
    pub placeholder: String,
    pub initial_text: String,
}

impl Textarea {
    /// A multi-line editable text field. Listen for edits with
    /// `.on_text_changed(...)`, same as [`crate::components::TextInput`].
    pub fn new(placeholder: impl Into<String>) -> Component {
        Component::Textarea(Self { placeholder: placeholder.into(), initial_text: String::new() })
    }

    /// Same as [`Textarea::new`], pre-filled with `initial_text`.
    pub fn with_text(placeholder: impl Into<String>, initial_text: impl Into<String>) -> Component {
        Component::Textarea(Self { placeholder: placeholder.into(), initial_text: initial_text.into() })
    }
}
