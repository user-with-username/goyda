use goyda::prelude::{page, stack, Component, Color};

#[page("/")]
pub fn counter_page() -> Component {
    let mut count = 0;

    stack! {
        direction: Vertical,
        spacing: 16,

        text { "Counter" }
            .font_size(24)
            .bold(),

        text { "Count: ", count }
            .color(Color::GRAY),

        button {
            text: "+1",
            on_click: count += 1,
        }
            .background(Color::GREEN)
            .px(4)
            .py(2)
            .rounded(2),

        button {
            text: "-1",
            on_click: count -= 1,
        }
            .background(Color::RED)
            .px(4)
            .py(2)
            .rounded(2),

        button {
            text: "Reset",
            on_click: count = 0,
        }
            .background(Color::GRAY)
            .px(4)
            .py(2)
            .rounded(2)
    }
    .p(24)
    .background(Color::WHITE)
}
