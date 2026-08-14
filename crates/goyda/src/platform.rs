//! Platform-independent entry points ([`navigate`], [`rerender`]) that work
//! the same way regardless of which target the app is built for.
//!
//! Enable exactly one of the `android` / `web` / `windows` features when
//! building a consumer crate.

pub use crate::listeners as active_listeners;

#[cfg(feature = "web")]
pub use crate::web::backend::WebBackend as ActiveBackend;

#[cfg(all(feature = "android", not(feature = "web")))]
pub use crate::android::backend::AndroidBackend as ActiveBackend;

#[cfg(all(feature = "windows", not(any(feature = "web", feature = "android"))))]
pub use crate::windows::backend::WindowsBackend as ActiveBackend;

/// Navigates the app to the `#[page(...)]` registered for `path`, falling
/// back to `"/"` if there's no exact match.
///
/// ```ignore
/// goyda::navigate("/about");
/// ```
#[cfg(feature = "web")]
pub use crate::web::navigate;

#[cfg(all(feature = "android", not(feature = "web")))]
pub use crate::android::navigate;

#[cfg(all(feature = "windows", not(any(feature = "web", feature = "android"))))]
pub use crate::windows::navigate;

/// Rebuilds and redisplays the currently mounted page in place, without
/// changing the route.
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
