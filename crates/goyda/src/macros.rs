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
                        let backend = &mut *(backend_ptr as *mut $crate::android::AndroidBackend);
                        let view = &*(view_ptr as *const <$crate::android::AndroidBackend as $crate::core::Backend>::PlatformView);
                        $crate::android::listeners::$event_name::attach(backend, view, callback);
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
                        let backend = &mut *(backend_ptr as *mut $crate::android::AndroidBackend);
                        let view = &*(view_ptr as *const <$crate::android::AndroidBackend as $crate::core::Backend>::PlatformView);
                        $crate::android::listeners::$event_name::attach(backend, view, callback);
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
                        let backend = &mut *(backend_ptr as *mut $crate::android::AndroidBackend);
                        let view = &*(view_ptr as *const <$crate::android::AndroidBackend as $crate::core::Backend>::PlatformView);
                        $crate::android::listeners::$event_name::attach(backend, view, callback);
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
                        let backend = &mut *(backend_ptr as *mut $crate::android::AndroidBackend);
                        let view = &*(view_ptr as *const <$crate::android::AndroidBackend as $crate::core::Backend>::PlatformView);
                        $crate::android::listeners::$event_name::attach(backend, view, callback);
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

    ($children:ident, $child:expr, $($tail:tt)+) => {
        $children.push($child);
        $crate::parse_children!($children, $($tail)+);
    };

    ($children:ident, $child:expr $(,)?) => {
        $children.push($child);
    };

    ($children:ident $(,)?) => {};
}

#[macro_export]
macro_rules! stack {
    (direction: $dir:ident, spacing: $space:expr, $($tail:tt)*) => {{
        let mut children = Vec::new();
        $crate::parse_children!(children, $($tail)*);
        $crate::Component::stack($crate::LayoutDirection::$dir, $space, children)
    }};
}
