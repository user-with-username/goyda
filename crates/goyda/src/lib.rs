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

pub use crate::components::{Component, LayoutDirection, Color, Modifier};
pub use crate::components::{Axis, Edge, StyleValue, StyleProperty};
pub use crate::macros::IntoString;
pub use crate::platform::navigate;

pub mod prelude {
    pub use goyda_derive::page;
    pub use crate::{stack, parse_children, asset, asset_ref};
    pub use crate::components::{Component, LayoutDirection, Color, Modifier, Asset};
    pub use crate::reactive::{Signal, Memo};
    pub use crate::navigate;
}

#[derive(Copy, Clone)]
pub struct Page {
    pub route: &'static str,
    pub factory: fn() -> Component,
}

impl Page {
    pub const fn new(route: &'static str, factory: fn() -> Component) -> Self {
        Self { route, factory }
    }
}

inventory::collect!(Page);

/// Finds the [`Page`] registered for `path` (via `#[page("...")]`), falling
/// back to whatever is registered at `"/"` if there's no exact match - the
/// same rule every backend's initial-route and [`navigate`] resolution uses.
pub fn find_page(path: &str) -> Option<&'static Page> {
    inventory::iter::<Page>
        .into_iter()
        .find(|p| p.route == path)
        .or_else(|| inventory::iter::<Page>.into_iter().find(|p| p.route == "/"))
}
