use goyda::prelude::{page, stack, Component, Color, asset, navigate};

#[page("/")]
pub fn counter_page() -> Component {
    let mut count = 0;
    let mut doubled = count * 2;

    stack! {
        direction: Vertical,
        spacing: 16,

        image { src: asset!("logo.svg") },

        text { "Count: ", count }
            .font_size(24)
            .bold(),

        text { "Doubled: ", doubled }
            .color(Color::GRAY),

        button {
            text: "-1",
            on_click: count -= 1,
        }
            .background(Color::RED)
            .px(4)
            .py(2)
            .rounded(2),

        button {
            text: "+1",
            on_click: count += 1,
        }
            .background(Color::GREEN)
            .px(4)
            .py(2)
            .rounded(2),

        button {
            text: "About this example",
            on_click: navigate("/about"),
        }
            .color(Color::GRAY)
            .underline()
    }
    .p(24)
}

#[page("/about")]
pub fn about_page() -> Component {
    stack! {
        direction: Vertical,
        spacing: 16,

        text { "count is a plain `let mut` - no signals, no setState to write by hand. Reassign it from on_click and the page re-renders." }
            .color(Color::GRAY),

        button {
            text: "Back",
            on_click: navigate("/"),
        }
            .background(Color::GRAY)
            .px(4)
            .py(2)
            .rounded(2)
    }
    .p(24)
}
