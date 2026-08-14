use goyda::prelude::{page, component, stack, Component, Color, Align, LayoutDirection, asset, navigate, theme};

#[component]
fn back_button(label: &'static str, to: &'static str) -> Component {
    stack! {
        direction: Vertical,
        spacing: 0,

        button {
            text: label,
            on_click: navigate(to),
        }
            .background(COLOR_MUTED())
            .px(4)
            .py(2)
            .rounded(2)
    }
}

theme! {
    Light, Dark, Solarized;

    COLOR_PRIMARY: Color::Custom(0xFF3949AB), Color::Custom(0xFF7986CB), Color::Custom(0xFFB58900);
    COLOR_MUTED: Color::GRAY, Color::Custom(0xFFB0B0B0), Color::Custom(0xFF93A1A1);
    COLOR_SUCCESS: Color::GREEN, Color::Custom(0xFF81C784), Color::Custom(0xFF859900);
    COLOR_DANGER: Color::Custom(0xFFE53935), Color::Custom(0xFFEF5350), Color::Custom(0xFFDC322F);
}

const FONT_SIZE_2XL: i32 = 24;

#[page("/")]
pub fn counter_page() -> Component {
    let mut count = 0;
    let mut doubled = count * 2;
    let mut history: Vec<i32> = Vec::new();

    stack! {
        direction: Vertical,
        spacing: 16,

        image { src: asset!("logo.svg") },

        text { "currrent: ", count }
            .color(COLOR_PRIMARY())
            .font_size(FONT_SIZE_2XL)
            .font("fonts/Inter-Bold.ttf"),

        text { "Doubled: ", doubled }
            .color(COLOR_MUTED()),

        text { "At zero: ", count == 0 }
            .color(COLOR_MUTED()),

        text { "Logged so far: ", history.len() }
            .color(COLOR_MUTED()),

        button {
            text: "+1",
            on_click: count += 1,
        }
            .background(COLOR_SUCCESS())
            .px(4)
            .py(2)
            .rounded(2),

        button {
            text: "-1",
            on_click: count -= 1,
        }
            .background(COLOR_DANGER())
            .px(4)
            .py(2)
            .rounded(2),

        button {
            text: "Log count",
            on_click: history.push(count),
        }
            .background(COLOR_PRIMARY())
            .px(4)
            .py(2)
            .rounded(2),

        button {
            text: "Reset to doubled",
            on_click: count = doubled,
        }
            .padding(8, 16)
            .border_color(COLOR_MUTED())
            .border_width(1)
            .opacity(0.8),

        button {
            text: "Go to About",
            on_click: navigate("/about"),
        }
            .background(COLOR_MUTED())
            .px(4)
            .py(2)
            .rounded(2),

        button {
            text: "Go to Widgets",
            on_click: navigate("/widgets"),
        }
            .background(COLOR_PRIMARY())
            .px(4)
            .py(2)
            .rounded(2),

        button {
            text: "Go to Styles",
            on_click: navigate("/styles"),
        }
            .background(COLOR_PRIMARY())
            .px(4)
            .py(2)
            .rounded(2)
    }
    .p(6)
    .background(COLOR_DANGER())
}

#[page("/styles")]
pub fn styles_page() -> Component {
    let mut plan = String::from("Free");

    let rows: Vec<Component> = (0..15)
        .map(|i| Component::text(move || format!("Scrollable row {i}")).color(COLOR_MUTED()))
        .collect();

    stack! {
        direction: Vertical,
        spacing: 12,

        text { "Styles & components" }
            .color(COLOR_PRIMARY())
            .font_size(FONT_SIZE_2XL)
            .bold(),

        Component::card(vec![
            Component::text(|| "Card content".to_string()).bold(),
            Component::text(|| "Centered, italic, fixed width".to_string())
                .italic()
                .text_align(Align::Center)
                .width(240),
        ]),

        Component::stack(LayoutDirection::Horizontal, 8, vec![
            Component::badge("NEW", Color::GREEN),
            Component::badge("3", Color::RED),
            Component::badge("BETA", Color::ORANGE),
        ]),

        Component::link("This is a link - click it", || {})
            .underline(),

        Component::text(|| "Strikethrough text".to_string())
            .strikethrough()
            .color(COLOR_MUTED()),

        Component::text(|| "Wide letter spacing".to_string())
            .letter_spacing(4),

        Component::text(|| "A very long line that should truncate with an ellipsis instead of wrapping".to_string())
            .width(200)
            .ellipsis(),

        Component::card(vec![
            Component::text(|| "Colored shadow card".to_string()),
        ])
            .shadow_color(COLOR_PRIMARY()),

        Component::overlay(vec![
            Component::stack(LayoutDirection::Vertical, 0, vec![]).background(COLOR_PRIMARY()).size(80).rounded(8),
            Component::badge("9", Color::RED).offset(56, 0).z_index(1),
        ]),

        Component::radio_button("plan", "Free", true)
            .on_checked_change(move |_| plan = "Free".into()),
        Component::radio_button("plan", "Pro", false)
            .on_checked_change(move |_| plan = "Pro".into()),
        text { "Selected plan: ", plan }
            .color(COLOR_MUTED()),

        Component::textarea("Write something long...")
            .height(70)
            .line_height(28),

        text { "Scrollable list (150px tall, mouse wheel to scroll):" }
            .color(COLOR_MUTED()),
        Component::scroll_view(LayoutDirection::Vertical, 4, rows)
            .height(150)
            .border_color(COLOR_MUTED())
            .border_width(1),

        back_button("Back", "/")
    }
    .p(6)
    .background(Color::WHITE)
}

#[page("/widgets")]
pub fn widgets_page() -> Component {
    let mut name = String::new();
    let mut accepted = false;
    let mut notifications = true;
    let mut loading = 0.35_f32;

    stack! {
        direction: Vertical,
        spacing: 16,

        text { "Widgets demo" }
            .color(COLOR_PRIMARY())
            .font_size(FONT_SIZE_2XL),

        Component::text_input("Type your name...")
            .on_text_changed(move |text| name = text),

        text { "Hello, ", name }
            .color(COLOR_MUTED()),

        Component::checkbox("Accept terms", false)
            .on_checked_change(move |checked| accepted = checked),

        text { "Accepted: ", accepted }
            .color(COLOR_MUTED()),

        Component::switch(true)
            .on_checked_change(move |checked| notifications = checked),

        text { "Notifications: ", notifications }
            .color(COLOR_MUTED()),

        text { "Drag to set: ", loading }
            .color(COLOR_MUTED()),

        Component::progress(move || loading)
            .on_value_changed(move |value| loading = value),

        button {
            text: "Next theme",
            on_click: next_theme(),
        }
            .background(COLOR_PRIMARY())
            .px(4)
            .py(2)
            .rounded(2),

        Component::spacer(24),

        text { "End of demo" }
            .color(COLOR_MUTED()),

        back_button("Back", "/")
    }
    .p(6)
    .background(Color::Custom(0xFFFFFFFF))
}

#[page("/about")]
pub fn about_page() -> Component {
    stack! {
        direction: Vertical,
        spacing: 16,

        text { "About" }
            .color(COLOR_PRIMARY())
            .font_size(FONT_SIZE_2XL),

        text { "This page lives at /about and was reached via goyda::navigate()." }
            .color(COLOR_MUTED()),

        button {
            text: "Back to counter",
            on_click: navigate("/"),
        }
            .background(COLOR_PRIMARY())
            .px(4)
            .py(2)
            .rounded(2)
    }
    .p(6)
    .background(COLOR_DANGER())
}
