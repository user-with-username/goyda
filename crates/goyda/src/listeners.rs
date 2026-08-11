//! Every `button { ... on_event: ... }` handler goyda knows about, defined
//! once per event via [`goyda_macros::define_listener`] with both an
//! `android` (JNI) and a `web` (DOM) implementation inline. The macro emits
//! a `pub mod on_<name>` per platform, each gated behind that platform's
//! feature, so exactly one is compiled into any given build - see
//! [`crate::platform`] for how `$crate::platform::active_listeners` picks
//! whichever one is active.

use goyda_macros::define_listener;

define_listener! {
    mod_name = click,
    callback = dyn Fn(Event),

    android {
        class_name = "goyda/internal/GoydaClickListener",
        interface_name = "android/view/View$OnClickListener",
        setter = "setOnClickListener",
        methods = [
            {
                name = "onClick",
                jni_sig = (("android/view/View") -> void),
                native_fn = native_on_click(env, this, _view: JObject<'a>) -> () {
                    if let Some(cb) = self::get_callback(&mut env, &this) {
                        cb(Event::Click);
                    }
                }
            }
        ]
    },

    web {
        events = [
            {
                dom_event = "click",
                handler = |_event: web_sys::MouseEvent, callback| {
                    callback(Event::Click);
                }
            }
        ]
    },
}

define_listener! {
    mod_name = long_click,
    callback = dyn Fn(Event),

    android {
        class_name = "goyda/internal/GoydaLongClickListener",
        interface_name = "android/view/View$OnLongClickListener",
        setter = "setOnLongClickListener",
        methods = [
            {
                name = "onLongClick",
                jni_sig = (("android/view/View") -> int),
                native_fn = native_on_long_click(env, this, _view: JObject<'a>) -> jboolean {
                    if let Some(cb) = self::get_callback(&mut env, &this) {
                        cb(Event::LongClick);
                    }
                    JNI_TRUE
                }
            }
        ]
    },

    // No single DOM event maps to "long click" - it needs a press-and-hold
    // timer shared across mousedown/touchstart (start) and
    // mouseup/mouseleave/touchend (cancel), so it uses the `custom` escape
    // hatch instead of the declarative `events` list.
    web {
        custom = |_backend, view, callback| {
            use std::cell::RefCell;
            use std::rc::Rc;
            use wasm_bindgen::closure::Closure;
            use wasm_bindgen::JsCast;

            const LONG_PRESS_MS: i32 = 500;

            fn window() -> web_sys::Window {
                web_sys::window().expect("goyda(web): no global `window`")
            }

            let fire_closure = Closure::<dyn FnMut()>::new(move || {
                callback(Event::LongClick);
            });
            let fire_fn: js_sys::Function = fire_closure.as_ref().clone().unchecked_into();
            fire_closure.forget();

            let timer_handle: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(None));

            let start_timer_handle = timer_handle.clone();
            let start_closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |_evt: web_sys::Event| {
                let id = window()
                    .set_timeout_with_callback_and_timeout_and_arguments_0(&fire_fn, LONG_PRESS_MS)
                    .ok();
                *start_timer_handle.borrow_mut() = id;
            });

            let cancel_timer_handle = timer_handle.clone();
            let cancel_closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |_evt: web_sys::Event| {
                if let Some(id) = cancel_timer_handle.borrow_mut().take() {
                    window().clear_timeout_with_handle(id);
                }
            });

            let element = &view.element;
            let _ = element.add_event_listener_with_callback("mousedown", start_closure.as_ref().unchecked_ref());
            let _ = element.add_event_listener_with_callback("touchstart", start_closure.as_ref().unchecked_ref());
            let _ = element.add_event_listener_with_callback("mouseup", cancel_closure.as_ref().unchecked_ref());
            let _ = element.add_event_listener_with_callback("mouseleave", cancel_closure.as_ref().unchecked_ref());
            let _ = element.add_event_listener_with_callback("touchend", cancel_closure.as_ref().unchecked_ref());

            start_closure.forget();
            cancel_closure.forget();
        }
    },
}

define_listener! {
    mod_name = checked_change,
    callback = dyn Fn(Event),

    android {
        class_name = "goyda/internal/GoydaCheckedChangeListener",
        interface_name = "android/widget/CompoundButton$OnCheckedChangeListener",
        setter = "setOnCheckedChangeListener",
        methods = [
            {
                name = "onCheckedChanged",
                jni_sig = (("android/widget/CompoundButton", int) -> void),
                native_fn = native_on_checked_changed(env, this, _view: JObject<'a>, checked: jboolean) -> () {
                    if let Some(cb) = self::get_callback(&mut env, &this) {
                        cb(Event::CheckedChanged(checked != 0));
                    }
                }
            }
        ]
    },

    web {
        events = [
            {
                dom_event = "change",
                handler = |event: web_sys::Event, callback| {
                    let checked = event
                        .target()
                        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|input| input.checked())
                        .unwrap_or(false);
                    callback(Event::CheckedChanged(checked));
                }
            }
        ]
    },
}

define_listener! {
    mod_name = focus_change,
    callback = dyn Fn(Event),

    android {
        class_name = "goyda/internal/GoydaFocusChangeListener",
        interface_name = "android/view/View$OnFocusChangeListener",
        setter = "setOnFocusChangeListener",
        methods = [
            {
                name = "onFocusChange",
                jni_sig = (("android/view/View", int) -> void),
                native_fn = native_on_focus_change(env, this, _view: JObject<'a>, has_focus: jboolean) -> () {
                    if let Some(cb) = self::get_callback(&mut env, &this) {
                        cb(Event::FocusChanged(has_focus != 0));
                    }
                }
            }
        ]
    },

    web {
        events = [
            {
                dom_event = "focus",
                handler = |_event: web_sys::FocusEvent, callback| {
                    callback(Event::FocusChanged(true));
                }
            },
            {
                dom_event = "blur",
                handler = |_event: web_sys::FocusEvent, callback| {
                    callback(Event::FocusChanged(false));
                }
            }
        ]
    },
}

define_listener! {
    mod_name = text_watcher,
    callback = dyn Fn(Event),

    android {
        class_name = "goyda/internal/GoydaTextWatcher",
        interface_name = "android/text/TextWatcher",
        setter = "addTextChangedListener",
        methods = [
            {
                name = "beforeTextChanged",
                jni_sig = (("java/lang/CharSequence", int, int, int) -> void),
                native_fn = native_before_text_changed(env, _this, _s: JObject<'a>, _start: jint, _count: jint, _after: jint) -> () {}
            },
            {
                name = "onTextChanged",
                jni_sig = (("java/lang/CharSequence", int, int, int) -> void),
                native_fn = native_on_text_changed(env, this, s: JObject<'a>, start: jint, before: jint, count: jint) -> () {
                    if let Some(cb) = self::get_callback(&mut env, &this) {
                        let jstr = ::jni::objects::JString::from(s);
                        let text: String = env.get_string(&jstr)
                            .map(|s| s.into())
                            .unwrap_or_default();
                        cb(Event::TextChanged {
                            text,
                            start: start as usize,
                            before: before as usize,
                            count: count as usize,
                        });
                    }
                }
            },
            {
                name = "afterTextChanged",
                jni_sig = (("android/text/Editable") -> void),
                native_fn = native_after_text_changed(env, this, _editable: JObject<'a>) -> () {}
            }
        ]
    },

    // The web `input` event doesn't expose Android's change deltas, so
    // `start`/`before` are always 0 and `count` is the new text's length
    // rather than the edit size - see Event::TextChanged's doc, not a
    // precise equivalent, just the closest approximation available.
    web {
        events = [
            {
                dom_event = "input",
                handler = |event: web_sys::Event, callback| {
                    let text = event
                        .target()
                        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|el| el.value())
                        .or_else(|| {
                            event
                                .target()
                                .and_then(|t| t.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
                                .map(|el| el.value())
                        })
                        .unwrap_or_default();
                    let count = text.chars().count();
                    callback(Event::TextChanged { text, start: 0, before: 0, count });
                }
            }
        ]
    },
}
