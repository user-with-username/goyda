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
    let mut history: Vec<i32> = Vec::new();

    stack! {
        direction: Vertical,
        spacing: 16,

        text { "Current: ", count }
            .color(COLOR_PRIMARY)
            .font_size(FONT_SIZE_2XL),

        text { "Doubled: ", doubled }
            .color(COLOR_MUTED),

        text { "At zero: ", count == 0 }
            .color(COLOR_MUTED),

        text { "Logged so far: ", history.len() }
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
            text: "Log count",
            on_click: history.push(count),
        }
            .background(COLOR_PRIMARY)
            .px(4)
            .py(2)
            .rounded(2),

        button {
            text: "Reset to doubled",
            on_click: count = doubled,
        }
            .padding(8, 16)
            .border_color(COLOR_MUTED)
            .border_width(1)
            .opacity(0.8)
    }
    .p(6)
    .background(COLOR_DANGER)
}