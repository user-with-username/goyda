//! Tests for `src/components/button.rs`.

use goyda::Component;
use goyda::components::Button;

#[test]
fn button_new_sets_text() {
    match Button::new("Click me") {
        Component::Button(btn) => assert_eq!(btn.text, "Click me"),
        _ => panic!("expected Component::Button"),
    }
}

#[test]
fn component_button_matches_button_new() {
    match Component::button("Save") {
        Component::Button(btn) => assert_eq!(btn.text, "Save"),
        _ => panic!("expected Component::Button"),
    }
}
