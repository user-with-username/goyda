//! Tests for the `theme!` macro in `src/macros.rs`.
//!
//! Shares the same process-wide theme index as `core_theme.rs`'s tests, but
//! that's a separate test binary (each `tests/*.rs` file compiles to its
//! own executable), so there's no cross-file interference. Every assertion
//! here still lives in one `#[test]` to avoid interleaving with itself.

use goyda::Color;
use goyda::prelude::*;

theme! {
    Light, Dark, Solarized;

    COLOR_A: Color::WHITE, Color::BLACK, Color::GRAY;
}

#[test]
fn generated_color_fn_and_next_theme_cycle_through_declared_variants() {
    set_theme(Light);
    assert!(matches!(COLOR_A(), Color::WHITE));

    next_theme();
    assert_eq!(theme_index(), Dark);
    assert!(matches!(COLOR_A(), Color::BLACK));

    next_theme();
    assert_eq!(theme_index(), Solarized);
    assert!(matches!(COLOR_A(), Color::GRAY));

    // Wraps back to the first variant after the last.
    next_theme();
    assert_eq!(theme_index(), Light);
}

#[test]
fn declared_variants_are_usize_constants_in_declaration_order() {
    assert_eq!(Light, 0);
    assert_eq!(Dark, 1);
    assert_eq!(Solarized, 2);
}
