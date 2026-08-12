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

    ($children:ident, $child:expr, $($tail:tt)+) => {
        $children.push($child);
        $crate::parse_children!($children, $($tail)+);
    };

    ($children:ident, $child:expr $(,)?) => {
        $children.push($child);
    };

    ($children:ident $(,)?) => {};
}

/// Embeds an asset's bytes into the binary at compile time (relative to the
/// crate's `assets/` directory), e.g. `asset!("logo.svg")` or
/// `asset!("fonts/Inter-Bold.ttf")`. A missing file is a compile error, and
/// the resulting [`Asset`](crate::components::Asset) needs no platform
/// filesystem/network access at runtime - it works identically on every
/// backend. Best for small-to-medium files (images, fonts, icons); for
/// large or rarely-used assets where binary size matters, use [`asset_ref!`]
/// instead.
#[macro_export]
macro_rules! asset {
    ($path:literal) => {
        $crate::components::Asset::embedded(
            $path,
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $path)),
        )
    };
}

/// Like [`asset!`], checks at compile time that the file exists under the
/// crate's `assets/` directory, but doesn't embed its bytes - the resulting
/// [`Asset`](crate::components::Asset) is resolved by each backend at
/// runtime instead (from the APK's packaged assets, or fetched by URL on
/// web), same as it always has been. Use this for large or dynamic assets
/// (video, audio, big image sets) that shouldn't bloat the binary.
#[macro_export]
macro_rules! asset_ref {
    ($path:literal) => {{
        const _: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $path));
        $crate::components::Asset::new($path)
    }};
}

#[macro_export]
macro_rules! stack {
    (direction: $dir:ident, spacing: $space:expr, $($tail:tt)*) => {{
        let mut children = Vec::new();
        $crate::parse_children!(children, $($tail)*);
        $crate::Component::stack($crate::LayoutDirection::$dir, $space, children)
    }};
}