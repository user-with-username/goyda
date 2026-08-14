//! Tests for `src/core/events.rs`.
//!
//! `Event`/`Update` are plain data enums with no derives beyond what's
//! needed to construct and pattern-match them, so this just confirms every
//! variant round-trips through construction and matching.

use goyda::core::events::{Event, Update};

#[test]
fn event_variants_construct_and_match() {
    assert!(matches!(Event::Click, Event::Click));
    assert!(matches!(Event::LongClick, Event::LongClick));
    assert!(matches!(
        Event::CheckedChanged(true),
        Event::CheckedChanged(true)
    ));
    assert!(matches!(
        Event::FocusChanged(false),
        Event::FocusChanged(false)
    ));
    assert!(matches!(Event::ValueChanged(0.5), Event::ValueChanged(v) if v == 0.5));

    let text_changed = Event::TextChanged {
        text: "hi".into(),
        start: 1,
        before: 2,
        count: 3,
    };
    match text_changed {
        Event::TextChanged {
            text,
            start,
            before,
            count,
        } => {
            assert_eq!(text, "hi");
            assert_eq!(start, 1);
            assert_eq!(before, 2);
            assert_eq!(count, 3);
        }
        _ => panic!("expected Event::TextChanged"),
    }
}

#[test]
fn update_variants_construct_and_match() {
    match Update::SetText("hello".into()) {
        Update::SetText(text) => assert_eq!(text, "hello"),
        _ => panic!("expected Update::SetText"),
    }

    match Update::SetProgress(0.75) {
        Update::SetProgress(value) => assert_eq!(value, 0.75),
        _ => panic!("expected Update::SetProgress"),
    }
}
