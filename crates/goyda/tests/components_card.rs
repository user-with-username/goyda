//! Tests for `src/components/card.rs`.

use goyda::components::Card;
use goyda::{Axis, Color, Component, Edge, StyleValue};

#[test]
fn card_wraps_children_in_a_pre_styled_vertical_stack() {
    let children = vec![Component::text(|| "content".into())];
    match Card::new(children) {
        Component::Styled { component, styles } => {
            match *component {
                Component::Stack(stack) => {
                    assert_eq!(stack.direction, goyda::LayoutDirection::Vertical);
                    assert_eq!(stack.spacing, 8);
                    assert_eq!(stack.children.len(), 1);
                }
                _ => panic!("expected a Stack underneath the Styled wrapper"),
            }

            assert_eq!(styles.len(), 5);
            assert!(matches!(styles[0].0, Axis::BackgroundColor));
            assert!(matches!(styles[0].1, StyleValue::Color(Color::WHITE)));
            assert!(matches!(styles[1].0, Axis::BorderRadius));
            assert!(matches!(styles[1].1, StyleValue::Spacing(8)));
            assert!(matches!(styles[2].0, Axis::Shadow));
            assert!(matches!(styles[2].1, StyleValue::Spacing(4)));
            assert!(matches!(styles[3].0, Axis::Padding(Edge::Horizontal)));
            assert!(matches!(styles[3].1, StyleValue::Length(v) if v == 16.0));
            assert!(matches!(styles[4].0, Axis::Padding(Edge::Vertical)));
            assert!(matches!(styles[4].1, StyleValue::Length(v) if v == 16.0));
        }
        _ => panic!("expected Component::Styled"),
    }
}
