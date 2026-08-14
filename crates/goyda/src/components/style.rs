pub use goyda_utils::{Align, Axis, Edge, SPACING, StyleProperty, StyleValue};

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
