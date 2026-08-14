//! Tests for `src/components/divider.rs`.

use goyda::Component;

#[test]
fn divider_new_produces_divider_variant() {
    match goyda::components::Divider::new() {
        Component::Divider(_) => {}
        _ => panic!("expected Component::Divider"),
    }
}

#[test]
fn component_divider_matches_new() {
    match Component::divider() {
        Component::Divider(_) => {}
        _ => panic!("expected Component::Divider"),
    }
}
