//! Tests for `src/components/layout.rs`.

use goyda::{Component, LayoutDirection};
use goyda::components::Stack;

#[test]
fn stack_new_sets_direction_spacing_and_children() {
    let children = vec![Component::text(|| "a".into()), Component::text(|| "b".into())];
    match Stack::new(LayoutDirection::Horizontal, 12, children) {
        Component::Stack(stack) => {
            assert_eq!(stack.direction, LayoutDirection::Horizontal);
            assert_eq!(stack.spacing, 12);
            assert_eq!(stack.children.len(), 2);
        }
        _ => panic!("expected Component::Stack"),
    }
}

#[test]
fn component_stack_matches_stack_new() {
    match Component::stack(LayoutDirection::Vertical, 4, vec![]) {
        Component::Stack(stack) => {
            assert_eq!(stack.direction, LayoutDirection::Vertical);
            assert_eq!(stack.spacing, 4);
            assert!(stack.children.is_empty());
        }
        _ => panic!("expected Component::Stack"),
    }
}
