//! Tests for `IntoString` in `src/macros.rs` - what `text { ... }` blocks
//! use to stringify each interpolated part.

use goyda::macros::IntoString;
use goyda::prelude::{Signal, Memo};

#[test]
fn display_types_format_via_display() {
    assert_eq!(42i32.to_string_reactive(), "42");
    assert_eq!("hi".to_string_reactive(), "hi");
    assert_eq!(true.to_string_reactive(), "true");
    assert_eq!(3.5f64.to_string_reactive(), "3.5");
}

#[test]
fn signal_reads_through_to_its_current_value() {
    let s = Signal::new(1);
    assert_eq!(s.to_string_reactive(), "1");
    s.set(2);
    assert_eq!(s.to_string_reactive(), "2");
}

#[test]
fn memo_reads_through_to_its_current_value() {
    let s = Signal::new(2);
    let doubled = {
        let s = s.clone();
        Memo::new(move || s.get() * 2)
    };
    assert_eq!(doubled.to_string_reactive(), "4");
    s.set(5);
    assert_eq!(doubled.to_string_reactive(), "10");
}
