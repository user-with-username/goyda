//! Tests for `src/components/link.rs`.

use goyda::{Axis, Color, Component, StyleValue};
use goyda::components::Link;
use goyda::core::events::Event;

#[test]
fn link_is_primary_colored_text_with_a_click_handler() {
    use std::cell::Cell;
    use std::rc::Rc;

    let clicked = Rc::new(Cell::new(false));
    let clicked_clone = clicked.clone();

    match Link::new("Click me", move || clicked_clone.set(true)) {
        Component::WithHandlers { component, handlers } => {
            assert_eq!(handlers.len(), 1);

            match *component {
                Component::Styled { component, styles } => {
                    match *component {
                        Component::Text(text) => assert_eq!((text.compute)(), "Click me"),
                        _ => panic!("expected a Text underneath the Styled wrapper"),
                    }
                    assert_eq!(styles.len(), 1);
                    assert!(matches!(styles[0].0, Axis::TextColor));
                    assert!(matches!(styles[0].1, StyleValue::Color(Color::PRIMARY)));
                }
                _ => panic!("expected Component::Styled underneath the WithHandlers wrapper"),
            }

            // The handler's callback (not its unsafe `attach` fn, which
            // needs a real platform backend) can be invoked directly.
            (handlers[0].callback)(Event::Click);
            assert!(clicked.get());
        }
        _ => panic!("expected Component::WithHandlers"),
    }
}
