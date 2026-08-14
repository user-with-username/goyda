//! Tests for `src/components/radio_button.rs`.

use goyda::Component;
use goyda::components::RadioButton;

#[test]
fn new_sets_group_label_and_selected() {
    match RadioButton::new("plan", "Free", true) {
        Component::RadioButton(rb) => {
            assert_eq!(rb.group, "plan");
            assert_eq!(rb.label, "Free");
            assert!(rb.selected);
        }
        _ => panic!("expected Component::RadioButton"),
    }
}

#[test]
fn component_radio_button_matches_new() {
    match Component::radio_button("plan", "Pro", false) {
        Component::RadioButton(rb) => {
            assert_eq!(rb.group, "plan");
            assert_eq!(rb.label, "Pro");
            assert!(!rb.selected);
        }
        _ => panic!("expected Component::RadioButton"),
    }
}
