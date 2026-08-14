//! Tests for `src/components/switch.rs`.

use goyda::Component;
use goyda::components::Switch;

#[test]
fn new_sets_checked() {
    match Switch::new(true) {
        Component::Switch(s) => assert!(s.checked),
        _ => panic!("expected Component::Switch"),
    }
}

#[test]
fn component_switch_matches_new() {
    match Component::switch(false) {
        Component::Switch(s) => assert!(!s.checked),
        _ => panic!("expected Component::Switch"),
    }
}
