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

        Component::text_input("Type your name...")
            .on_text_changed(move |text| name = text),

    
        text { "Hello, ", name }
            .color(Color::GRAY),

        Component::checkbox("Accept terms", false)
            .on_checked_change(move |checked| accepted = checked),

        text { "Accepted: ", accepted }
            .color(Color::GRAY),

        Component::switch(true)
            .on_checked_change(move |checked| notifications = checked),

        text { "Notifications: ", notifications }
            .color(Color::GRAY),

        Component::progress(move || loading)
            .on_value_changed(move |value| loading = value),

        text { "Drag the bar above to set: ", loading }
            .color(Color::GRAY)
    }
    .p(24)
}
