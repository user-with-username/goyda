//! App-wide theme state: not just light/dark - a [`crate::theme`] block can
//! declare any number of named variants (`theme! { Light, Dark, Solarized; ... }`),
//! each color getting one value per variant. The "current variant" is just
//! its declaration-order index, tracked here as a single global; each
//! platform's bootstrap seeds it from whatever the OS reports (`uiMode` on
//! android, `AppsUseLightTheme` on windows, `prefers-color-scheme` on web -
//! see each platform's `detect_theme_mode`) before the first page renders,
//! and [`set_theme`]/[`cycle_theme`] (or the `next_theme()` function
//! `theme!` generates) are how an app changes it after that.

use std::sync::atomic::{AtomicUsize, Ordering};

/// What the OS itself reports - only meaningful as a *starting point* (see
/// each platform's `detect_theme_mode`), mapped to index `0`/`1`. A
/// `theme!` block with more than two variants is expected to declare the
/// light-equivalent one first and the dark-equivalent one second, same as
/// the two-variant case, so this OS reading still lands on the right one;
/// anything past index `1` is only ever reached by the app's own
/// [`set_theme`]/[`cycle_theme`] calls, never by OS detection.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeMode {
    Light,
    Dark,
}

static THEME_INDEX: AtomicUsize = AtomicUsize::new(0);

/// Which `theme!` variant (by declaration order) every themed color
/// currently resolves against.
pub fn theme_index() -> usize {
    THEME_INDEX.load(Ordering::Relaxed)
}

/// Seeds the initial index from the OS's light/dark reading - called once
/// by each platform's bootstrap, before any page is mounted, so it
/// deliberately skips the [`set_theme`] rerender (there's nothing on
/// screen yet to rerender).
pub fn init_theme_mode(mode: ThemeMode) {
    THEME_INDEX.store(if mode == ThemeMode::Dark { 1 } else { 0 }, Ordering::Relaxed);
}

/// Switches to the `theme!` variant at `index` (by declaration order - the
/// constants a `theme!` block generates, e.g. `set_theme(Dark)`) and
/// rerenders the current page in place, the same "tear down and rebuild
/// the mounted page" every platform's `navigate` already does for a route
/// change - so every `theme!` color picks up the change immediately, with
/// no manual "refresh" step on the caller's side. An out-of-range `index`
/// just clamps to the last declared variant (see the `theme!`-generated
/// color functions), rather than panicking.
pub fn set_theme(index: usize) {
    THEME_INDEX.store(index, Ordering::Relaxed);
    crate::platform::rerender();
}

/// Advances to the next of `count` declared themes, wrapping back to `0`
/// after the last. `count` comes from `theme!`'s own generated
/// `next_theme()` - call that instead of this directly unless you're
/// managing the variant count yourself.
pub fn cycle_theme(count: usize) {
    if count == 0 {
        return;
    }
    set_theme((theme_index() + 1) % count);
}
