pub enum Event {
    Click,
    LongClick,
    CheckedChanged(bool),
    FocusChanged(bool),
    TextChanged {
        text: String,
        start: usize,
        before: usize,
        count: usize,
    },
}

pub enum Update {
    SetText(String),
}