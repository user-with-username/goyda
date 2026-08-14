//! Tests for `src/components/progress.rs`.

use goyda::Component;
use goyda::components::Progress;

#[test]
fn new_wraps_compute_closure() {
    match Progress::new(|| 0.5) {
        Component::Progress(p) => assert_eq!((p.compute)(), 0.5),
        _ => panic!("expected Component::Progress"),
    }
}

#[test]
fn value_is_a_fixed_non_reactive_value() {
    match Progress::value(0.25) {
        Component::Progress(p) => {
            assert_eq!((p.compute)(), 0.25);
            assert_eq!((p.compute)(), 0.25);
        }
        _ => panic!("expected Component::Progress"),
    }
}

#[test]
fn component_progress_matches_new() {
    match Component::progress(|| 0.75) {
        Component::Progress(p) => assert_eq!((p.compute)(), 0.75),
        _ => panic!("expected Component::Progress"),
    }
}
