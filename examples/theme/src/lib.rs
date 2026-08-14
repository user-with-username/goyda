use goyda::prelude::{page, stack, Component, Color, theme};

theme! {
    Light, Dark, Solarized;

    COLOR_BG: Color::WHITE, Color::Custom(0xFF1E1E1E), Color::Custom(0xFFFDF6E3);
    COLOR_TEXT: Color::Custom(0xFF212121), Color::WHITE, Color::Custom(0xFF586E75);
    COLOR_ACCENT: Color::Custom(0xFF3949AB), Color::Custom(0xFF7986CB), Color::Custom(0xFFB58900);
}

#[page("/")]
pub fn theme_page() -> Component {
    stack! {
        direction: Vertical,
        spacing: 16,

        text { "Light, Dark, Solarized - pick one." }
            .color(COLOR_TEXT())
            .font_size(20)
            .bold(),

        text { "Every COLOR_* call re-resolves against whichever theme is active - no re-render logic to write." }
            .color(COLOR_TEXT()),

        button {
            text: "Next theme",
            on_click: next_theme(),
        }
            .background(COLOR_ACCENT())
            .px(4)
            .py(2)
            .rounded(2)
    }
    .p(24)
    .background(COLOR_BG())
}
