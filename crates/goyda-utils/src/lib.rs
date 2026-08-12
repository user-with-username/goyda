//! Backend-agnostic value types and pure resolution logic shared by every
//! goyda platform backend (color palette, style values/spacing, asset path
//! handling). No platform dependencies (no `jni`, no `wasm-bindgen`) live
//! here on purpose - anything that needs them belongs in `goyda::android` /
//! `goyda::web` instead, not here.

pub mod asset;
pub mod color;
pub mod style;

pub use asset::Asset;
pub use color::Color;
pub use style::{Axis, Edge, StyleProperty, StyleValue, SPACING};
