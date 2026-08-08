pub use jni;
pub use inventory;

pub mod components;
pub mod core;
pub mod macros;
pub mod reactive;
pub mod android;

pub use crate::components::{Component, LayoutDirection, Color, Modifier};
pub use crate::components::{Axis, Edge, StyleValue, StyleProperty};
pub use crate::macros::IntoString;

pub mod prelude {
    pub use goyda_derive::page;
    pub use crate::{stack, parse_children};
    pub use crate::components::{Component, LayoutDirection, Color, Modifier};
    pub use crate::reactive::{Signal, Memo};
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
