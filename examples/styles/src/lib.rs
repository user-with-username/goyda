use goyda::prelude::{page, stack, Component, Color, Align, LayoutDirection};

#[page("/")]
pub fn styles_page() -> Component {
    let mut plan = String::from("Free");

    let rows: Vec<Component> = (0..15)
        .map(|i| Component::text(move || format!("Scrollable row {i}")).color(Color::GRAY))
        .collect();

    stack! {
        direction: Vertical,
        spacing: 12,

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

        Component::text(|| "A very long line that should truncate with an ellipsis instead of wrapping".to_string())
            .width(200)
            .ellipsis(),

        radio_button { group: "plan", label: "Free", selected: true }
            .on_checked_change(move |_| plan = "Free".into()),
        radio_button { group: "plan", label: "Pro", selected: false }
            .on_checked_change(move |_| plan = "Pro".into()),
        text { "Selected plan: ", plan }
            .color(Color::GRAY),

        text { "Scrollable list (150px tall, mouse wheel to scroll):" }
            .color(Color::GRAY),
        Component::scroll_view(LayoutDirection::Vertical, 4, rows)
            .height(150)
            .border_color(Color::GRAY)
            .border_width(1)
    }
    .p(24)
}
