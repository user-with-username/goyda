//! Tests for `src/components/textarea.rs`.

use goyda::Component;
use goyda::components::Textarea;

#[test]
fn new_sets_placeholder_and_empty_initial_text() {
    match Textarea::new("Notes") {
        Component::Textarea(t) => {
            assert_eq!(t.placeholder, "Notes");
            assert_eq!(t.initial_text, "");
        }
        _ => panic!("expected Component::Textarea"),
    }
}

#[test]
fn with_text_prefills_initial_text() {
    match Textarea::with_text("Notes", "Draft") {
        Component::Textarea(t) => {
            assert_eq!(t.placeholder, "Notes");
            assert_eq!(t.initial_text, "Draft");
        }
        _ => panic!("expected Component::Textarea"),
    }
}

#[test]
fn component_textarea_matches_new() {
    match Component::textarea("Bio") {
        Component::Textarea(t) => assert_eq!(t.placeholder, "Bio"),
        _ => panic!("expected Component::Textarea"),
    }
}
