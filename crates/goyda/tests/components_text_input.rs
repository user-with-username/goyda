//! Tests for `src/components/text_input.rs`.

use goyda::Component;
use goyda::components::TextInput;

#[test]
fn new_sets_placeholder_and_empty_initial_text() {
    match TextInput::new("Name") {
        Component::TextInput(input) => {
            assert_eq!(input.placeholder, "Name");
            assert_eq!(input.initial_text, "");
        }
        _ => panic!("expected Component::TextInput"),
    }
}

#[test]
fn with_text_prefills_initial_text() {
    match TextInput::with_text("Name", "Alice") {
        Component::TextInput(input) => {
            assert_eq!(input.placeholder, "Name");
            assert_eq!(input.initial_text, "Alice");
        }
        _ => panic!("expected Component::TextInput"),
    }
}

#[test]
fn component_text_input_matches_new() {
    match Component::text_input("Search") {
        Component::TextInput(input) => {
            assert_eq!(input.placeholder, "Search");
            assert_eq!(input.initial_text, "");
        }
        _ => panic!("expected Component::TextInput"),
    }
}
