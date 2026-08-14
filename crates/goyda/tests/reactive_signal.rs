//! Tests for `src/reactive.rs`'s public API (`Signal`, `Memo`,
//! `create_effect`). `Signal::new_keyed`'s persist-across-remount behavior
//! already has dedicated tests inline in `src/reactive.rs` itself.

use goyda::reactive::{Signal, Memo, create_effect};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn get_and_set_read_back_the_current_value() {
    let s = Signal::new(1);
    assert_eq!(s.get(), 1);
    s.set(5);
    assert_eq!(s.get(), 5);
}

#[test]
fn update_mutates_in_place() {
    let s = Signal::new(vec![1, 2]);
    s.update(|v| v.push(3));
    assert_eq!(s.get(), vec![1, 2, 3]);
}

#[test]
fn clone_shares_the_same_underlying_value() {
    let s1 = Signal::new(1);
    let s2 = s1.clone();
    s1.set(9);
    assert_eq!(s2.get(), 9);
}

#[test]
fn set_notifies_subscribed_effects() {
    let s = Signal::new(0);
    let seen = Rc::new(Cell::new(0));
    let seen_clone = seen.clone();
    let s_clone = s.clone();
    create_effect(move || seen_clone.set(s_clone.get()));

    // create_effect runs its body once immediately.
    assert_eq!(seen.get(), 0);

    s.set(7);
    assert_eq!(seen.get(), 7);
}

#[test]
fn call_only_notifies_when_the_value_actually_changed() {
    let s = Signal::new(1);
    let runs = Rc::new(Cell::new(0));
    let runs_clone = runs.clone();
    let s_clone = s.clone();
    create_effect(move || {
        s_clone.get();
        runs_clone.set(runs_clone.get() + 1);
    });
    assert_eq!(runs.get(), 1);

    s.call(|v| *v = 1); // unchanged
    assert_eq!(runs.get(), 1);

    s.call(|v| *v = 2); // changed
    assert_eq!(runs.get(), 2);
}

#[test]
fn memo_recomputes_when_a_read_signal_changes() {
    let s = Signal::new(2);
    let doubled = {
        let s = s.clone();
        Memo::new(move || s.get() * 2)
    };
    assert_eq!(doubled.get(), 4);

    s.set(10);
    assert_eq!(doubled.get(), 20);
}

#[test]
fn memo_does_not_recompute_when_an_unrelated_signal_changes() {
    let tracked = Signal::new(1);
    let untracked = Signal::new(100);
    let calls = Rc::new(Cell::new(0));
    let calls_clone = calls.clone();

    let memo = {
        let tracked = tracked.clone();
        Memo::new(move || {
            calls_clone.set(calls_clone.get() + 1);
            tracked.get()
        })
    };
    assert_eq!(memo.get(), 1);
    // Memo::new runs its closure twice up front: once directly for the
    // initial value, once more (subscribing to any signal it reads this
    // time) via run_with_effect.
    let calls_after_construction = calls.get();
    assert_eq!(calls_after_construction, 2);

    untracked.set(999);
    assert_eq!(calls.get(), calls_after_construction, "memo should not recompute for a signal it never read");

    tracked.set(2);
    assert_eq!(memo.get(), 2);
    assert_eq!(calls.get(), calls_after_construction + 1);
}

#[test]
fn create_effect_runs_immediately_and_on_every_change() {
    let s = Signal::new(0);
    let log = Rc::new(RefCell::new(Vec::new()));
    let log_clone = log.clone();
    let s_clone = s.clone();
    create_effect(move || log_clone.borrow_mut().push(s_clone.get()));

    s.set(1);
    s.set(2);

    assert_eq!(&*log.borrow(), &[0, 1, 2]);
}
