//! Tests for `src/core/theme.rs`.
//!
//! `THEME_INDEX` is a single process-wide atomic, so every assertion lives
//! in one `#[test]` to avoid interleaving with other tests that might touch
//! the same state (`cargo test` runs tests in parallel threads by default).

use goyda::{theme_index, set_theme, cycle_theme, ThemeMode};

#[test]
fn theme_state_transitions() {
    set_theme(0);
    assert_eq!(theme_index(), 0);

    set_theme(2);
    assert_eq!(theme_index(), 2);

    // cycle_theme(count) advances by one and wraps back to 0 after `count`.
    set_theme(0);
    cycle_theme(3);
    assert_eq!(theme_index(), 1);
    cycle_theme(3);
    assert_eq!(theme_index(), 2);
    cycle_theme(3);
    assert_eq!(theme_index(), 0);

    // A zero-variant theme has nothing to cycle to - index is left as-is.
    set_theme(1);
    cycle_theme(0);
    assert_eq!(theme_index(), 1);

    // init_theme_mode() maps Light/Dark to declaration indices 0/1.
    goyda::core::theme::init_theme_mode(ThemeMode::Dark);
    assert_eq!(theme_index(), 1);
    goyda::core::theme::init_theme_mode(ThemeMode::Light);
    assert_eq!(theme_index(), 0);
}

#[test]
fn theme_mode_equality() {
    assert_eq!(ThemeMode::Light, ThemeMode::Light);
    assert_ne!(ThemeMode::Light, ThemeMode::Dark);
}
