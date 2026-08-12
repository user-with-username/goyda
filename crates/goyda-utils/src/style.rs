use crate::asset::Asset;
use crate::color::Color;

/// Spacing scale indices resolve into this table (in the framework's
/// abstract unit space - backends apply their own platform scaling on top).
pub const SPACING: [f32; 13] = [
    0.0, 2.0, 4.0, 8.0, 12.0, 16.0, 20.0, 24.0, 32.0, 40.0, 48.0, 64.0, 80.0,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edge {
    All,
    Horizontal,
    Vertical,
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    TextColor,
    BackgroundColor,
    BorderColor,
    BorderWidth,
    BorderRadius,
    Shadow,
    Opacity,
    FontSize,
    FontFamily,
    Padding(Edge),
    Margin(Edge),
}

#[derive(Debug, Clone)]
pub enum StyleValue {
    Color(Color),
    Length(f32),
    Spacing(usize),
    Number(f32),
    Asset(Asset),
}

#[derive(Debug, Clone)]
pub struct StyleProperty(pub Axis, pub StyleValue);

/// Looks up a spacing-scale index in [`SPACING`].
pub fn resolve_spacing(scale: usize) -> Option<f32> {
    SPACING.get(scale).copied()
}

/// Resolves a numeric [`StyleValue`] (a literal length, or a spacing-scale
/// index) to a raw value in the framework's abstract unit space. Backends
/// apply their own platform scaling on top (e.g. android multiplies by a
/// density factor and rounds to a pixel `int`; the web backend uses the
/// value directly as CSS `px`).
pub fn resolve_length(value: &StyleValue) -> Option<f32> {
    match value {
        StyleValue::Length(v) => Some(*v),
        StyleValue::Spacing(scale) => resolve_spacing(*scale),
        _ => None,
    }
}
