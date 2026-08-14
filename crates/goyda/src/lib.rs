pub use inventory;
#[cfg(feature = "android")]
pub use jni;
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

pub use crate::components::{Axis, Edge, StyleProperty, StyleValue};
pub use crate::components::{Color, Component, LayoutDirection, Modifier};
pub use crate::core::theme::{ThemeMode, cycle_theme, set_theme, theme_index};
pub use crate::macros::IntoString;
pub use crate::platform::{navigate, rerender};

pub mod prelude {
    pub use crate::components::{Align, Asset, Color, Component, LayoutDirection, Modifier};
    pub use crate::navigate;
    pub use crate::reactive::{Memo, Signal};
    pub use crate::{ThemeMode, cycle_theme, set_theme, theme_index};
    pub use crate::{asset, asset_ref, parse_children, stack, theme};
    pub use goyda_derive::{component, page};
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
