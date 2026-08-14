use crate::asset::Asset;
use crate::color::Color;

/// The spacing scale used by [`StyleValue::Spacing`] indices.
pub const SPACING: [f32; 13] = [
    0.0, 2.0, 4.0, 8.0, 12.0, 16.0, 20.0, 24.0, 32.0, 40.0, 48.0, 64.0, 80.0,
];

/// A side (or combination of sides) of a component's box, used with
/// [`Axis::Padding`] and [`Axis::Margin`].
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

/// A stylable property of a component, paired with a value in
/// [`StyleProperty`].
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
    FontWeight,
    FontStyle,
    TextAlign,
    Padding(Edge),
    Margin(Edge),
    Width,
    Height,
    /// Cross-axis alignment of a `Stack`'s children.
    AlignItems,
    /// Main-axis distribution of a `Stack`'s children.
    JustifyContent,
    LineHeight,
    LetterSpacing,
    Underline,
    Strikethrough,
    /// Truncates overflowing text to a single line with a trailing "…".
    TextOverflowEllipsis,
    /// Clips content that overflows the component's bounds.
    Clip,
    /// A secondary shadow color, paired with [`Axis::Shadow`]'s size.
    ShadowColor,
    /// Horizontal offset from the top-left corner of an enclosing `Overlay`.
    OffsetX,
    /// Vertical offset from the top-left corner of an enclosing `Overlay`.
    OffsetY,
    /// Paint order among the siblings of an enclosing `Overlay` — higher
    /// draws on top.
    ZIndex,
}

/// Alignment used by [`Axis::TextAlign`], [`Axis::AlignItems`], and
/// [`Axis::JustifyContent`]. Not every variant applies to every axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
    SpaceBetween,
}

/// The value of a style property.
#[derive(Debug, Clone)]
pub enum StyleValue {
    Color(Color),
    Length(f32),
    Spacing(usize),
    Number(f32),
    Asset(Asset),
    Bool(bool),
    Align(Align),
}

/// A style property and its value, e.g. `StyleProperty(Axis::Opacity,
/// StyleValue::Number(0.5))`.
#[derive(Debug, Clone)]
pub struct StyleProperty(pub Axis, pub StyleValue);

/// Looks up a spacing-scale index in [`SPACING`].
pub fn resolve_spacing(scale: usize) -> Option<f32> {
    SPACING.get(scale).copied()
}

/// Resolves a numeric [`StyleValue`] — a literal length or a spacing-scale
/// index — to its raw value. `None` for values that aren't a length.
pub fn resolve_length(value: &StyleValue) -> Option<f32> {
    match value {
        StyleValue::Length(v) => Some(*v),
        StyleValue::Spacing(scale) => resolve_spacing(*scale),
        _ => None,
    }
}
