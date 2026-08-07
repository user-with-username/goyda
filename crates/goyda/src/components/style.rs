use crate::components::Color;

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
    Padding(Edge),
    Margin(Edge),
}

#[derive(Debug, Clone)]
pub enum StyleValue {
    Color(Color),
    Length(f32),
    Spacing(usize),
    Number(f32),
}

#[derive(Debug, Clone)]
pub struct StyleProperty(pub Axis, pub StyleValue);

#[macro_export]
macro_rules! style_methods {
    ($( $(#[$meta:meta])* $name:ident ( $arg:ty ) => $axis:expr, $ctor:path );* $(;)?) => {
        impl $crate::components::Component {
            $(
                $(#[$meta])*
                pub fn $name(self, value: $arg) -> Self {
                    self.style($crate::components::StyleProperty($axis, $ctor(value)))
                }
            )*
        }
    };
}
