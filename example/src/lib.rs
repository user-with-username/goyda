use goyda::prelude::{page, stack, Component, Color};

const COLOR_PRIMARY: Color = Color::Custom(0xFF3949AB);
const COLOR_MUTED: Color = Color::GRAY;
const COLOR_SUCCESS: Color = Color::GREEN;
const COLOR_DANGER: Color = Color::Custom(0xFFE53935);

const FONT_SIZE_2XL: i32 = 24;

#[page("/")]
pub fn counter_page() -> Component {
    let mut count = 0;
    let mut doubled = count * 2;
    eprintln!("Rendering counter_page with count");

    stack! {
        direction: Vertical,
        spacing: 16,

        text { "Current: ", count }
            .color(COLOR_PRIMARY)
            .font_size(FONT_SIZE_2XL),

        text { "Doubled: ", doubled }
            .color(COLOR_MUTED),

        button {
            text: "+1",
            on_click: count += 1,
        }
            .background(COLOR_SUCCESS)
            .px(4)
            .py(2)
            .rounded(2),

        button {
            text: "-1",
            on_click: count -= 1,
        }
            .background(COLOR_DANGER)
            .px(4)
            .py(2)
            .rounded(2),

        button {
            text: "Reset",
            on_click: count = 0,
        }
            .padding(8, 16)
            .border_color(COLOR_MUTED)
            .border_width(1)
            .opacity(0.8)
    }
    .p(6)
    .background(COLOR_DANGER)
}
