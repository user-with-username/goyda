//! Resolves the single [`crate::core::Backend`] implementation that the
//! `button { ... on_click: ... }` handler-attachment codegen in
//! [`crate::macros`] binds against.
//!
//! The UI macros are expanded inside consumer crates (e.g. `example`), so
//! they cannot pick a backend type at their own compile time based on which
//! target is being built - they need a single, stable path to reach for.
//! `ActiveBackend` is that path: whichever platform feature is enabled on
//! `goyda` decides what it points to.
//!
//! `active_listeners` always points at [`crate::listeners`]: every event
//! there is defined once via `goyda_macros::define_listener!` with an
//! `android`, `web`, and `windows` implementation, each behind its own
//! `#[cfg(feature = ...)]`, so the module itself picks the right one
//! internally - no per-platform re-export needed here.
//!
//! Enable exactly one of the `android` / `web` / `windows` features when
//! building a consumer crate (the `goy` CLI already does this for you via
//! `--features goyda/android`, `--features goyda/web`, or
//! `--features goyda/windows`).

pub use crate::listeners as active_listeners;

#[cfg(feature = "web")]
pub use crate::web::backend::WebBackend as ActiveBackend;

#[cfg(all(feature = "android", not(feature = "web")))]
pub use crate::android::backend::AndroidBackend as ActiveBackend;

#[cfg(all(feature = "windows", not(any(feature = "web", feature = "android"))))]
pub use crate::windows::backend::WindowsBackend as ActiveBackend;

/// Switches the mounted app to whichever `#[page(...)]` is registered for
/// `path` (falling back to `"/"` if there's no exact match) - the same
/// single entry point on every backend, so `goyda::navigate("/about")` reads
/// identically regardless of which platform feature is enabled. A missing
/// route is a silent no-op in release builds and a console/stderr warning in
/// debug builds, matching how an unmatched initial route is already handled.
#[cfg(feature = "web")]
pub use crate::web::navigate;

#[cfg(all(feature = "android", not(feature = "web")))]
pub use crate::android::navigate;

#[cfg(all(feature = "windows", not(any(feature = "web", feature = "android"))))]
pub use crate::windows::navigate;

/// Re-renders whatever page is currently mounted, in place - no route
/// change, no history entry, just "run the current page's factory again
/// and swap it in". [`crate::core::theme::set_theme_mode`] is what actually
/// calls this (so a runtime theme switch shows up immediately); exposed
/// here too for anything else that wants the same "state changed
/// out-of-band, rebuild what's on screen" behavior.
#[cfg(feature = "web")]
pub use crate::web::rerender;

#[cfg(all(feature = "android", not(feature = "web")))]
pub use crate::android::rerender;

#[cfg(all(feature = "windows", not(any(feature = "web", feature = "android"))))]
pub use crate::windows::rerender;

#[cfg(not(any(feature = "android", feature = "web", feature = "windows")))]
compile_error!(
    "goyda: no platform backend selected. Enable exactly one of the \"android\", \"web\", or \
     \"windows\" features on the goyda crate (the `goy` CLI does this automatically via \
     `--features goyda/android`, `--features goyda/web`, or `--features goyda/windows`)."
);
