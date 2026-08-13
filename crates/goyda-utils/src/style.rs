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
    FontWeight,
    FontStyle,
    TextAlign,
    Padding(Edge),
    Margin(Edge),
    Width,
    Height,
    /// Cross-axis alignment of a [`Stack`](https://docs.rs/goyda)'s
    /// children (`Start`/`Center`/`End`/`Stretch` - `Stretch` is the
    /// existing default every backend already applies when this axis isn't
    /// set).
    AlignItems,
    /// Main-axis distribution of a `Stack`'s children (`Start`/`Center`/
    /// `End`/`SpaceBetween`).
    JustifyContent,
    LineHeight,
    LetterSpacing,
    Underline,
    Strikethrough,
    /// Truncates to a single line with a trailing "…" instead of wrapping/
    /// overflowing.
    TextOverflowEllipsis,
    /// Forces content that overflows this component's bounds to be clipped
    /// - mostly matters on the web backend, where the CSS default is
    /// `overflow: visible`; android/windows already clip a control's
    /// children to its own bounds unconditionally (see
    /// `crate::components::ScrollView`'s doc comment), so this is a no-op
    /// there.
    Clip,
    /// A secondary shadow color (paired with [`Axis::Shadow`]'s size) -
    /// defaults to a neutral gray when unset.
    ShadowColor,
    /// Only meaningful on a direct child of an
    /// [`Overlay`](https://docs.rs/goyda) - horizontal/vertical offset from
    /// the overlay's top-left corner, taking the child out of normal flow
    /// (`position: absolute` in CSS terms).
    OffsetX,
    OffsetY,
    /// Only meaningful on a direct child of an `Overlay` - paint order
    /// among overlay siblings (higher draws on top). Ties break by
    /// insertion order, matching how DOM/view stacking already works when
    /// this is left unset.
    ZIndex,
}

/// Shared by [`Axis::TextAlign`], [`Axis::AlignItems`], and
/// [`Axis::JustifyContent`] - not every variant is meaningful for every
/// axis (`TextAlign` never produces `Stretch`/`SpaceBetween`, e.g.), but one
/// enum keeps the style value type small instead of one bespoke enum per
/// axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
    SpaceBetween,
}

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
