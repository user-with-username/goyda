//! Tests for `src/components/text.rs`.

use goyda::Component;

#[test]
fn text_new_wraps_compute_closure() {
    let component = Component::text(|| "hello".to_string());
    match component {
        Component::Text(text) => assert_eq!((text.compute)(), "hello"),
        _ => panic!("expected Component::Text"),
    }
}

#[test]
fn text_compute_reruns_each_call() {
    use std::cell::Cell;
    use std::rc::Rc;

    let calls = Rc::new(Cell::new(0));
    let calls_clone = calls.clone();
    let component = Component::text(move || {
        calls_clone.set(calls_clone.get() + 1);
        "x".to_string()
    });

    match component {
        Component::Text(text) => {
            (text.compute)();
            (text.compute)();
            assert_eq!(calls.get(), 2);
        }
        _ => panic!("expected Component::Text"),
    }
}
