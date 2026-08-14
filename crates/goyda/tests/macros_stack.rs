//! Tests for the `stack!`/`parse_children!` DSL in `src/macros.rs`.

use goyda::prelude::*;
use goyda::{Axis, Color, StyleValue};

#[test]
fn stack_sets_direction_and_spacing_on_the_wrapping_component() {
    let s = stack! {
        direction: Horizontal,
        spacing: 8,
        text { "a" }
    };
    match s {
        Component::Stack(stack) => {
            assert_eq!(stack.direction, LayoutDirection::Horizontal);
            assert_eq!(stack.spacing, 8);
            assert_eq!(stack.children.len(), 1);
        }
        _ => panic!("expected Component::Stack"),
    }
}

#[test]
fn text_item_concatenates_parts_and_reruns_on_each_call() {
    let count = 3;
    let s = stack! {
        direction: Vertical,
        spacing: 0,
        text { "Count: ", count }
    };
    match s {
        Component::Stack(stack) => match &stack.children[0] {
            Component::Text(text) => assert_eq!((text.compute)(), "Count: 3"),
            _ => panic!("expected Component::Text"),
        },
        _ => panic!("expected Component::Stack"),
    }
}

#[test]
fn text_item_with_method_chain_applies_styles() {
    let s = stack! {
        direction: Vertical,
        spacing: 0,
        text { "hi" }.color(Color::RED).bold()
    };
    match s {
        Component::Stack(stack) => {
            let styles = match &stack.children[0] {
                Component::Styled { styles, .. } => styles,
                _ => panic!("expected Component::Styled"),
            };
            assert_eq!(styles.len(), 2);
            assert!(matches!(styles[0].0, Axis::TextColor));
            assert!(matches!(styles[1].0, Axis::FontWeight));
        }
        _ => panic!("expected Component::Stack"),
    }
}

#[test]
fn button_item_without_handler_is_a_plain_button() {
    let s = stack! {
        direction: Vertical,
        spacing: 0,
        button { text: "Go" }
    };
    match s {
        Component::Stack(stack) => match &stack.children[0] {
            Component::Button(btn) => assert_eq!(btn.text, "Go"),
            _ => panic!("expected Component::Button"),
        },
        _ => panic!("expected Component::Stack"),
    }
}

#[test]
fn button_item_with_handler_attaches_the_action() {
    use std::cell::Cell;
    use std::rc::Rc;

    let clicked = Rc::new(Cell::new(false));
    let clicked_clone = clicked.clone();

    let s = stack! {
        direction: Vertical,
        spacing: 0,
        button {
            text: "Go",
            on_click: clicked_clone.set(true),
        }
    };
    match s {
        Component::Stack(stack) => match &stack.children[0] {
            Component::WithHandlers {
                component,
                handlers,
            } => {
                assert!(matches!(**component, Component::Button(_)));
                assert_eq!(handlers.len(), 1);
                (handlers[0].callback)(goyda::core::events::Event::Click);
                assert!(clicked.get());
            }
            _ => panic!("expected Component::WithHandlers"),
        },
        _ => panic!("expected Component::Stack"),
    }
}

#[test]
fn button_item_with_handler_and_method_chain_applies_both() {
    let s = stack! {
        direction: Vertical,
        spacing: 0,
        button {
            text: "Go",
            on_click: (),
        }.background(Color::GREEN)
    };
    match s {
        Component::Stack(stack) => match &stack.children[0] {
            Component::Styled { component, styles } => {
                assert!(matches!(**component, Component::WithHandlers { .. }));
                assert_eq!(styles.len(), 1);
            }
            _ => panic!("expected Component::Styled wrapping WithHandlers"),
        },
        _ => panic!("expected Component::Stack"),
    }
}

#[test]
fn image_item_builds_component_image() {
    let s = stack! {
        direction: Vertical,
        spacing: 0,
        image { src: asset!("test_fixture.txt") }
    };
    match s {
        Component::Stack(stack) => match &stack.children[0] {
            Component::Image(img) => assert_eq!(img.asset.path(), "test_fixture.txt"),
            _ => panic!("expected Component::Image"),
        },
        _ => panic!("expected Component::Stack"),
    }
}

#[test]
fn text_input_checkbox_switch_progress_radio_button_textarea_items() {
    let s = stack! {
        direction: Vertical,
        spacing: 0,
        text_input { placeholder: "Name" },
        checkbox { label: "Accept", checked: true },
        switch { checked: true },
        progress { value: || 0.5_f32 },
        radio_button { group: "g", label: "A", selected: true },
        textarea { placeholder: "Notes" }
    };

    match s {
        Component::Stack(stack) => {
            assert_eq!(stack.children.len(), 6);
            match &stack.children[0] {
                Component::TextInput(t) => assert_eq!(t.placeholder, "Name"),
                _ => panic!("expected TextInput"),
            }
            match &stack.children[1] {
                Component::Checkbox(c) => {
                    assert_eq!(c.label, "Accept");
                    assert!(c.checked);
                }
                _ => panic!("expected Checkbox"),
            }
            match &stack.children[2] {
                Component::Switch(s) => assert!(s.checked),
                _ => panic!("expected Switch"),
            }
            match &stack.children[3] {
                Component::Progress(p) => assert_eq!((p.compute)(), 0.5),
                _ => panic!("expected Progress"),
            }
            match &stack.children[4] {
                Component::RadioButton(r) => {
                    assert_eq!(r.group, "g");
                    assert_eq!(r.label, "A");
                    assert!(r.selected);
                }
                _ => panic!("expected RadioButton"),
            }
            match &stack.children[5] {
                Component::Textarea(t) => assert_eq!(t.placeholder, "Notes"),
                _ => panic!("expected Textarea"),
            }
        }
        _ => panic!("expected Component::Stack"),
    }
}

#[test]
fn widget_item_with_method_chain_applies_styles() {
    let s = stack! {
        direction: Vertical,
        spacing: 0,
        text_input { placeholder: "Name" }.width(100)
    };
    match s {
        Component::Stack(stack) => {
            let styles = match &stack.children[0] {
                Component::Styled { styles, .. } => styles,
                _ => panic!("expected Component::Styled"),
            };
            assert!(matches!(styles[0].0, Axis::Width));
            assert!(matches!(styles[0].1, StyleValue::Length(v) if v == 100.0));
        }
        _ => panic!("expected Component::Stack"),
    }
}

#[test]
fn plain_expression_children_are_pushed_as_is() {
    let s = stack! {
        direction: Vertical,
        spacing: 0,
        Component::divider(),
        Component::spacer(4)
    };
    match s {
        Component::Stack(stack) => {
            assert_eq!(stack.children.len(), 2);
            assert!(matches!(stack.children[0], Component::Divider(_)));
            assert!(matches!(stack.children[1], Component::Spacer(_)));
        }
        _ => panic!("expected Component::Stack"),
    }
}
