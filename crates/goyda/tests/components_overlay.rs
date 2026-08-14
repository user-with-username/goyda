//! Tests for `src/components/overlay.rs`.

use goyda::Component;
use goyda::components::Overlay;

#[test]
fn new_sets_children() {
    let children = vec![Component::text(|| "a".into()), Component::text(|| "b".into())];
    match Overlay::new(children) {
        Component::Overlay(o) => assert_eq!(o.children.len(), 2),
        _ => panic!("expected Component::Overlay"),
    }
}

#[test]
fn component_overlay_matches_new() {
    match Component::overlay(vec![]) {
        Component::Overlay(o) => assert!(o.children.is_empty()),
        _ => panic!("expected Component::Overlay"),
    }
}
