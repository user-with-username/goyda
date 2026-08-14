//! Tests for `src/components/checkbox.rs`.

use goyda::Component;
use goyda::components::Checkbox;

#[test]
fn new_sets_label_and_checked() {
    match Checkbox::new("Accept", true) {
        Component::Checkbox(cb) => {
            assert_eq!(cb.label, "Accept");
            assert!(cb.checked);
        }
        _ => panic!("expected Component::Checkbox"),
    }
}

#[test]
fn component_checkbox_matches_new() {
    match Component::checkbox("Subscribe", false) {
        Component::Checkbox(cb) => {
            assert_eq!(cb.label, "Subscribe");
            assert!(!cb.checked);
        }
        _ => panic!("expected Component::Checkbox"),
    }
}
