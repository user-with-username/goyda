//! Tests for `src/components/badge.rs`.

use goyda::components::Badge;
use goyda::{Axis, Color, Component, Edge, StyleValue};

#[test]
fn badge_is_pre_styled_colored_text() {
    match Badge::new("NEW", Color::GREEN) {
        Component::Styled { component, styles } => {
            match *component {
                Component::Text(text) => assert_eq!((text.compute)(), "NEW"),
                _ => panic!("expected a Text underneath the Styled wrapper"),
            }

            assert_eq!(styles.len(), 6);
            assert!(matches!(styles[0].0, Axis::TextColor));
            assert!(matches!(styles[0].1, StyleValue::Color(Color::WHITE)));
            assert!(matches!(styles[1].0, Axis::BackgroundColor));
            assert!(matches!(styles[1].1, StyleValue::Color(Color::GREEN)));
            assert!(matches!(styles[2].0, Axis::FontSize));
            assert!(matches!(styles[2].1, StyleValue::Length(v) if v == 12.0));
            assert!(matches!(styles[3].0, Axis::Padding(Edge::Horizontal)));
            assert!(matches!(styles[3].1, StyleValue::Length(v) if v == 8.0));
            assert!(matches!(styles[4].0, Axis::Padding(Edge::Vertical)));
            assert!(matches!(styles[4].1, StyleValue::Length(v) if v == 2.0));
            assert!(matches!(styles[5].0, Axis::BorderRadius));
            assert!(matches!(styles[5].1, StyleValue::Spacing(999)));
        }
        _ => panic!("expected Component::Styled"),
    }
}
