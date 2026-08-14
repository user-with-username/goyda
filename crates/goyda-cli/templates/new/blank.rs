use goyda::prelude::{page, stack, Component, Color};

#[page("/")]
pub fn home_page() -> Component {
    stack! {
        direction: Vertical,
        spacing: 16,

        text { "Hello, Goyda!" }
            .font_size(24)
            .bold(),

        text { "Edit src/lib.rs and press r to hot-reload." }
            .color(Color::GRAY)
    }
    .p(24)
    .background(Color::WHITE)
}
