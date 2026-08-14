#[cfg(feature = "android")]
pub use jni;
pub use inventory;
#[cfg(feature = "web")]
pub use wasm_bindgen;

pub mod components;
pub mod core;
pub mod listeners;
pub mod macros;
pub mod platform;
pub mod reactive;

#[cfg(feature = "android")]
pub mod android;
#[cfg(feature = "web")]
pub mod web;
#[cfg(feature = "windows")]
pub mod windows;

pub use crate::components::{Component, LayoutDirection, Color, Modifier};
pub use crate::components::{Axis, Edge, StyleValue, StyleProperty};
pub use crate::macros::IntoString;
pub use crate::platform::{navigate, rerender};
pub use crate::core::theme::{ThemeMode, theme_index, set_theme, cycle_theme};

pub mod prelude {
    pub use goyda_derive::{page, component};
    pub use crate::{stack, parse_children, asset, asset_ref, theme};
    pub use crate::components::{Component, LayoutDirection, Color, Modifier, Asset, Align};
    pub use crate::reactive::{Signal, Memo};
    pub use crate::navigate;
    pub use crate::{ThemeMode, theme_index, set_theme, cycle_theme};
}

/// A route registered with `#[page("...")]`, pairing a path with the
/// component that renders it.
#[derive(Copy, Clone)]
pub struct Page {
    pub route: &'static str,
    pub factory: fn() -> Component,
}

impl Page {
    /// Creates a page entry for `route`, rendered by calling `factory`.
    pub const fn new(route: &'static str, factory: fn() -> Component) -> Self {
        Self { route, factory }
    }
}

inventory::collect!(Page);

/// Looks up the [`Page`] registered for `path`, falling back to the page
/// registered at `"/"` if there's no exact match.
///
/// ```ignore
/// if let Some(page) = goyda::find_page("/settings") {
///     let component = (page.factory)();
/// }
/// ```
pub fn find_page(path: &str) -> Option<&'static Page> {
    inventory::iter::<Page>
        .into_iter()
        .find(|p| p.route == path)
        .or_else(|| inventory::iter::<Page>.into_iter().find(|p| p.route == "/"))
}
