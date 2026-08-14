//! Tests for `src/components/spacer.rs`.

use goyda::Component;
use goyda::components::Spacer;

#[test]
fn new_sets_size() {
    match Spacer::new(24) {
        Component::Spacer(s) => assert_eq!(s.size, 24),
        _ => panic!("expected Component::Spacer"),
    }
}

#[test]
fn component_spacer_matches_new() {
    match Component::spacer(8) {
        Component::Spacer(s) => assert_eq!(s.size, 8),
        _ => panic!("expected Component::Spacer"),
    }
}
