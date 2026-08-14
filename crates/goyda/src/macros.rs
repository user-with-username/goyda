/// Converts a value (including a [`Signal`](crate::reactive::Signal) or
/// [`Memo`](crate::reactive::Memo)) to the `String` it should render as
/// inside `text { ... }` blocks.
pub trait IntoString {
    fn to_string_reactive(&self) -> String;
}

impl<T: std::fmt::Display> IntoString for T {
    fn to_string_reactive(&self) -> String { format!("{}", self) }
}

impl<T: std::fmt::Display + Copy + 'static> IntoString for crate::reactive::Signal<T> {
    fn to_string_reactive(&self) -> String { format!("{}", self.get()) }
}

impl<T: std::fmt::Display + Copy + 'static> IntoString for crate::reactive::Memo<T> {
    fn to_string_reactive(&self) -> String { format!("{}", self.get()) }
}

/// Declares a set of named theme variants and the colors that vary between
/// them. The first two variants should be a light and a dark equivalent, in
/// that order, so OS theme detection picks a sensible default.
///
/// Each color becomes a function (e.g. `COLOR_PRIMARY()`) returning the
/// value for whichever variant is currently active, and a `next_theme()`
/// function is generated for cycling between variants. To jump to a
/// specific variant instead, use [`crate::set_theme`].
///
/// ```ignore
/// theme! {
///     Light, Dark, Solarized;
///
///     COLOR_PRIMARY: Color::Custom(0xFF3949AB), Color::Custom(0xFF5C6BC0), Color::Custom(0xFFB58900);
///     COLOR_MUTED: Color::GRAY, Color::Custom(0xFF9AA0A6), Color::Custom(0xFF93A1A1);
/// }
/// // ... .background(COLOR_PRIMARY()) ...
/// // ... on_click: next_theme() ...
/// // ... on_click: goyda::set_theme(Dark) ...
/// ```
#[macro_export]
macro_rules! theme {
    (
        $( $variant:ident ),+ $(,)? ;
        $( $name:ident : $( $value:expr ),+ );+ $(;)?
    ) => {
        $crate::__theme_indices!(0usize; $($variant),+);

        #[allow(dead_code)]
        const __THEME_VARIANT_COUNT: usize = [$(stringify!($variant)),+].len();

        /// Switches to the next theme variant declared by [`theme!`],
        /// wrapping around after the last one, and rerenders the current
        /// page.
        pub fn next_theme() {
            $crate::core::theme::cycle_theme(__THEME_VARIANT_COUNT);
        }

        $(
            #[allow(non_snake_case)]
            pub fn $name() -> $crate::Color {
                let variants: &[fn() -> $crate::Color] = &[ $( || $value ),+ ];
                variants[$crate::core::theme::theme_index().min(variants.len() - 1)]()
            }
        )+
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __theme_indices {
    ($idx:expr; $head:ident $(, $tail:ident)*) => {
        #[allow(non_upper_case_globals)]
        pub const $head: usize = $idx;
        $crate::__theme_indices!($idx + 1; $($tail),*);
    };
    ($idx:expr;) => {};
}

/// Expands the child list inside a [`crate::stack!`] into a
/// `Vec<Component>`. Used internally by `stack!`; call `stack!` instead of
/// using this macro directly.
#[macro_export]
macro_rules! parse_children {
    ($children:ident, text { $($part:expr),+ $(,)? } $( . $method:ident ( $($args:tt)* ) )+ , $($tail:tt)*) => {
        let child = $crate::Component::text(move || {
            use $crate::macros::IntoString;
            let mut s = String::new();
            $( s.push_str(&$part.to_string_reactive()); )+
            s
        }) $( . $method ( $($args)* ) )+ ;
        $children.push(child);
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, text { $($part:expr),+ $(,)? } $( . $method:ident ( $($args:tt)* ) )+ $(,)?) => {
        let child = $crate::Component::text(move || {
            use $crate::macros::IntoString;
            let mut s = String::new();
            $( s.push_str(&$part.to_string_reactive()); )+
            s
        }) $( . $method ( $($args)* ) )+ ;
        $children.push(child);
    };

    ($children:ident, text { $($part:expr),+ $(,)? }, $($tail:tt)*) => {
        $children.push($crate::Component::text(move || {
            use $crate::macros::IntoString;
            let mut s = String::new();
            $( s.push_str(&$part.to_string_reactive()); )+
            s
        }));
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, text { $($part:expr),+ $(,)? } $(,)?) => {
        $children.push($crate::Component::text(move || {
            use $crate::macros::IntoString;
            let mut s = String::new();
            $( s.push_str(&$part.to_string_reactive()); )+
            s
        }));
    };

    ($children:ident, button { text: $txt:expr, $event_name:ident : $action:expr $(,)? } $( . $method:ident ( $($args:tt)* ) )+ , $($tail:tt)*) => {
        let child = $crate::Component::WithHandlers {
            component: Box::new($crate::Component::button($txt)),
            handlers: vec![
                $crate::components::Handler {
                    attach: |backend_ptr, view_ptr, callback| unsafe {
                        let backend = &mut *(backend_ptr as *mut $crate::platform::ActiveBackend);
                        let view = &*(view_ptr as *const <$crate::platform::ActiveBackend as $crate::core::Backend>::PlatformView);
                        $crate::platform::active_listeners::$event_name::attach(backend, view, callback);
                    },
                    callback: std::rc::Rc::new(move |_e| {
                        let mut _action_closure = || { $action };
                        _action_closure();
                    }),
                }
            ],
        } $( . $method ( $($args)* ) )+ ;
        $children.push(child);
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, button { text: $txt:expr, $event_name:ident : $action:expr $(,)? } $( . $method:ident ( $($args:tt)* ) )+ $(,)?) => {
        let child = $crate::Component::WithHandlers {
            component: Box::new($crate::Component::button($txt)),
            handlers: vec![
                $crate::components::Handler {
                    attach: |backend_ptr, view_ptr, callback| unsafe {
                        let backend = &mut *(backend_ptr as *mut $crate::platform::ActiveBackend);
                        let view = &*(view_ptr as *const <$crate::platform::ActiveBackend as $crate::core::Backend>::PlatformView);
                        $crate::platform::active_listeners::$event_name::attach(backend, view, callback);
                    },
                    callback: std::rc::Rc::new(move |_e| {
                        let mut _action_closure = || { $action };
                        _action_closure();
                    }),
                }
            ],
        } $( . $method ( $($args)* ) )+ ;
        $children.push(child);
    };

    ($children:ident, button { text: $txt:expr, $event_name:ident : $action:expr $(,)? }, $($tail:tt)*) => {
        $children.push($crate::Component::WithHandlers {
            component: Box::new($crate::Component::button($txt)),
            handlers: vec![
                $crate::components::Handler {
                    attach: |backend_ptr, view_ptr, callback| unsafe {
                        let backend = &mut *(backend_ptr as *mut $crate::platform::ActiveBackend);
                        let view = &*(view_ptr as *const <$crate::platform::ActiveBackend as $crate::core::Backend>::PlatformView);
                        $crate::platform::active_listeners::$event_name::attach(backend, view, callback);
                    },
                    callback: std::rc::Rc::new(move |_e| {
                        let mut _action_closure = || { $action };
                        _action_closure();
                    }),
                }
            ],
        });
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, button { text: $txt:expr, $event_name:ident : $action:expr $(,)? } $(,)?) => {
        $children.push($crate::Component::WithHandlers {
            component: Box::new($crate::Component::button($txt)),
            handlers: vec![
                $crate::components::Handler {
                    attach: |backend_ptr, view_ptr, callback| unsafe {
                        let backend = &mut *(backend_ptr as *mut $crate::platform::ActiveBackend);
                        let view = &*(view_ptr as *const <$crate::platform::ActiveBackend as $crate::core::Backend>::PlatformView);
                        $crate::platform::active_listeners::$event_name::attach(backend, view, callback);
                    },
                    callback: std::rc::Rc::new(move |_e| {
                        let mut _action_closure = || { $action };
                        _action_closure();
                    }),
                }
            ],
        });
    };

    ($children:ident, button { text: $txt:expr $(,)? }, $($tail:tt)*) => {
        $children.push($crate::Component::button($txt));
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, button { text: $txt:expr $(,)? } $(,)?) => {
        $children.push($crate::Component::button($txt));
    };

    ($children:ident, image { src: $src:expr $(,)? } $( . $method:ident ( $($args:tt)* ) )+ , $($tail:tt)*) => {
        $children.push($crate::Component::image($src) $( . $method ( $($args)* ) )+ );
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, image { src: $src:expr $(,)? } $( . $method:ident ( $($args:tt)* ) )+ $(,)?) => {
        $children.push($crate::Component::image($src) $( . $method ( $($args)* ) )+ );
    };

    ($children:ident, image { src: $src:expr $(,)? }, $($tail:tt)*) => {
        $children.push($crate::Component::image($src));
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, image { src: $src:expr $(,)? } $(,)?) => {
        $children.push($crate::Component::image($src));
    };

    ($children:ident, text_input { placeholder: $ph:expr $(,)? } $( . $method:ident ( $($args:tt)* ) )+ , $($tail:tt)*) => {
        $children.push($crate::Component::text_input($ph) $( . $method ( $($args)* ) )+ );
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, text_input { placeholder: $ph:expr $(,)? } $( . $method:ident ( $($args:tt)* ) )+ $(,)?) => {
        $children.push($crate::Component::text_input($ph) $( . $method ( $($args)* ) )+ );
    };

    ($children:ident, text_input { placeholder: $ph:expr $(,)? }, $($tail:tt)*) => {
        $children.push($crate::Component::text_input($ph));
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, text_input { placeholder: $ph:expr $(,)? } $(,)?) => {
        $children.push($crate::Component::text_input($ph));
    };

    ($children:ident, textarea { placeholder: $ph:expr $(,)? } $( . $method:ident ( $($args:tt)* ) )+ , $($tail:tt)*) => {
        $children.push($crate::Component::textarea($ph) $( . $method ( $($args)* ) )+ );
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, textarea { placeholder: $ph:expr $(,)? } $( . $method:ident ( $($args:tt)* ) )+ $(,)?) => {
        $children.push($crate::Component::textarea($ph) $( . $method ( $($args)* ) )+ );
    };

    ($children:ident, textarea { placeholder: $ph:expr $(,)? }, $($tail:tt)*) => {
        $children.push($crate::Component::textarea($ph));
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, textarea { placeholder: $ph:expr $(,)? } $(,)?) => {
        $children.push($crate::Component::textarea($ph));
    };

    ($children:ident, checkbox { label: $label:expr, checked: $checked:expr $(,)? } $( . $method:ident ( $($args:tt)* ) )+ , $($tail:tt)*) => {
        $children.push($crate::Component::checkbox($label, $checked) $( . $method ( $($args)* ) )+ );
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, checkbox { label: $label:expr, checked: $checked:expr $(,)? } $( . $method:ident ( $($args:tt)* ) )+ $(,)?) => {
        $children.push($crate::Component::checkbox($label, $checked) $( . $method ( $($args)* ) )+ );
    };

    ($children:ident, checkbox { label: $label:expr, checked: $checked:expr $(,)? }, $($tail:tt)*) => {
        $children.push($crate::Component::checkbox($label, $checked));
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, checkbox { label: $label:expr, checked: $checked:expr $(,)? } $(,)?) => {
        $children.push($crate::Component::checkbox($label, $checked));
    };

    ($children:ident, radio_button { group: $group:expr, label: $label:expr, selected: $selected:expr $(,)? } $( . $method:ident ( $($args:tt)* ) )+ , $($tail:tt)*) => {
        $children.push($crate::Component::radio_button($group, $label, $selected) $( . $method ( $($args)* ) )+ );
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, radio_button { group: $group:expr, label: $label:expr, selected: $selected:expr $(,)? } $( . $method:ident ( $($args:tt)* ) )+ $(,)?) => {
        $children.push($crate::Component::radio_button($group, $label, $selected) $( . $method ( $($args)* ) )+ );
    };

    ($children:ident, radio_button { group: $group:expr, label: $label:expr, selected: $selected:expr $(,)? }, $($tail:tt)*) => {
        $children.push($crate::Component::radio_button($group, $label, $selected));
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, radio_button { group: $group:expr, label: $label:expr, selected: $selected:expr $(,)? } $(,)?) => {
        $children.push($crate::Component::radio_button($group, $label, $selected));
    };

    ($children:ident, switch { checked: $checked:expr $(,)? } $( . $method:ident ( $($args:tt)* ) )+ , $($tail:tt)*) => {
        $children.push($crate::Component::switch($checked) $( . $method ( $($args)* ) )+ );
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, switch { checked: $checked:expr $(,)? } $( . $method:ident ( $($args:tt)* ) )+ $(,)?) => {
        $children.push($crate::Component::switch($checked) $( . $method ( $($args)* ) )+ );
    };

    ($children:ident, switch { checked: $checked:expr $(,)? }, $($tail:tt)*) => {
        $children.push($crate::Component::switch($checked));
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, switch { checked: $checked:expr $(,)? } $(,)?) => {
        $children.push($crate::Component::switch($checked));
    };

    ($children:ident, progress { value: $value:expr $(,)? } $( . $method:ident ( $($args:tt)* ) )+ , $($tail:tt)*) => {
        $children.push($crate::Component::progress($value) $( . $method ( $($args)* ) )+ );
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, progress { value: $value:expr $(,)? } $( . $method:ident ( $($args:tt)* ) )+ $(,)?) => {
        $children.push($crate::Component::progress($value) $( . $method ( $($args)* ) )+ );
    };

    ($children:ident, progress { value: $value:expr $(,)? }, $($tail:tt)*) => {
        $children.push($crate::Component::progress($value));
        $crate::parse_children!($children, $($tail)*);
    };

    ($children:ident, progress { value: $value:expr $(,)? } $(,)?) => {
        $children.push($crate::Component::progress($value));
    };

    ($children:ident, $child:expr, $($tail:tt)+) => {
        $children.push($child);
        $crate::parse_children!($children, $($tail)+);
    };

    ($children:ident, $child:expr $(,)?) => {
        $children.push($child);
    };

    ($children:ident $(,)?) => {};
}

/// Embeds a file from the crate's `assets/` directory into the binary at
/// compile time, producing an [`Asset`](crate::components::Asset). A
/// missing file is a compile error. Best for small-to-medium files (images,
/// fonts, icons); for large or rarely-used assets, use [`crate::asset_ref!`]
/// instead.
///
/// ```ignore
/// let logo = asset!("logo.svg");
/// ```
#[macro_export]
macro_rules! asset {
    ($path:literal) => {
        $crate::components::Asset::embedded(
            $path,
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $path)),
        )
    };
}

/// References a file in the crate's `assets/` directory without embedding
/// it into the binary, producing an [`Asset`](crate::components::Asset)
/// that's loaded at runtime. Use this for large or dynamic assets (video,
/// audio, big image sets) instead of [`asset!`].
///
/// ```ignore
/// let clip = asset_ref!("intro.mp4");
/// ```
#[macro_export]
macro_rules! asset_ref {
    ($path:literal) => {{
        const _: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $path));
        $crate::components::Asset::new($path)
    }};
}

/// Builds a [`Component`](crate::Component) that lays out its children
/// along an axis, with spacing between them.
///
/// ```ignore
/// stack! {
///     direction: Vertical,
///     spacing: 8.0,
///     text { "Hello" },
///     button { text: "Click me", on_click: do_something() },
///     text_input { placeholder: "Name" }.on_text_changed(|t| ..),
///     checkbox { label: "Accept", checked: false }.on_checked_change(|c| ..),
/// }
/// ```
#[macro_export]
macro_rules! stack {
    (direction: $dir:ident, spacing: $space:expr, $($tail:tt)*) => {{
        let mut children = Vec::new();
        $crate::parse_children!(children, $($tail)*);
        $crate::Component::stack($crate::LayoutDirection::$dir, $space, children)
    }};
}