//! Tests for `src/components/scroll_view.rs`.

use goyda::{Component, LayoutDirection};
use goyda::components::ScrollView;

#[test]
fn new_sets_direction_spacing_and_children() {
    let children = vec![Component::text(|| "row".into())];
    match ScrollView::new(LayoutDirection::Vertical, 4, children) {
        Component::ScrollView(sv) => {
            assert_eq!(sv.direction, LayoutDirection::Vertical);
            assert_eq!(sv.spacing, 4);
            assert_eq!(sv.children.len(), 1);
        }
        _ => panic!("expected Component::ScrollView"),
    }
}

#[test]
fn component_scroll_view_matches_new() {
    match Component::scroll_view(LayoutDirection::Horizontal, 2, vec![]) {
        Component::ScrollView(sv) => {
            assert_eq!(sv.direction, LayoutDirection::Horizontal);
            assert_eq!(sv.spacing, 2);
        }
        _ => panic!("expected Component::ScrollView"),
    }
}
