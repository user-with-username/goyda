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
    ValueChanged(f32),
}

pub enum Update {
    SetText(String),
    SetProgress(f32),
}
