//! Shared value types used throughout goyda: colors, style properties and
//! spacing, and asset references.

pub mod asset;
pub mod color;
pub mod style;

pub use asset::Asset;
pub use color::Color;
pub use style::{Align, Axis, Edge, SPACING, StyleProperty, StyleValue};
