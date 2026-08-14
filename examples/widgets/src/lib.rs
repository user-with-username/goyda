use goyda::prelude::{page, stack, Component, Color};

#[page("/")]
pub fn widgets_page() -> Component {
    let mut name = String::new();
    let mut accepted = false;
    let mut notifications = true;
    let mut loading = 0.35_f32;

    stack! {
        direction: Vertical,
        spacing: 16,

        text_input { placeholder: "Type your name..." }
            .on_text_changed(move |text| name = text),

        text { "Hello, ", name }
            .color(Color::GRAY),

        checkbox { label: "Accept terms", checked: false }
            .on_checked_change(move |checked| accepted = checked),

        text { "Accepted: ", accepted }
            .color(Color::GRAY),

        switch { checked: true }
            .on_checked_change(move |checked| notifications = checked),

        text { "Notifications: ", notifications }
            .color(Color::GRAY),

        progress { value: move || loading }
            .on_value_changed(move |value| loading = value),

        text { "Drag the bar above to set: ", loading }
            .color(Color::GRAY)
    }
    .p(24)
}
