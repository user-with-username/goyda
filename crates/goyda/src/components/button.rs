use crate::components::Component;

pub struct Button {
    pub text: String,
}

impl Button {
    pub fn new(text: impl Into<String>) -> Component {
        Component::Button(Self { text: text.into() })
    }
}
