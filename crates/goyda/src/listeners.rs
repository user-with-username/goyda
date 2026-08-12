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

    windows {
        custom = |_backend, view, callback| {
            use std::cell::Cell;
            use std::rc::Rc;
            use windows_sys::Win32::UI::WindowsAndMessaging::{WM_LBUTTONDOWN, WM_LBUTTONUP};

            let pressed = Rc::new(Cell::new(false));
            crate::windows::register_raw_hook(view.hwnd, Rc::new(move |msg, _wparam, _lparam| {
                match msg {
                    WM_LBUTTONDOWN => pressed.set(true),
                    WM_LBUTTONUP => {
                        if pressed.replace(false) {
                            callback(Event::Click);
                        }
                    }
                    _ => {}
                }
            }));
        }
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

    windows {
        custom = |_backend, view, callback| {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                KillTimer, SetTimer, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_TIMER,
            };

            const LONG_PRESS_MS: u32 = 500;
            const TIMER_ID: usize = 0xC0FFEE;

            let hwnd = view.hwnd;
            crate::windows::register_raw_hook(hwnd, std::rc::Rc::new(move |msg, wparam, _lparam| unsafe {
                match msg {
                    WM_LBUTTONDOWN => {
                        SetTimer(hwnd, TIMER_ID, LONG_PRESS_MS, None);
                    }
                    WM_LBUTTONUP => {
                        KillTimer(hwnd, TIMER_ID);
                    }
                    WM_TIMER if wparam == TIMER_ID => {
                        KillTimer(hwnd, TIMER_ID);
                        callback(Event::LongClick);
                    }
                    _ => {}
                }
            }));
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

    // No native checkbox widget exists in this backend (see
    // `windows/state.rs`'s `ControlKind`) - like the android/web arms above
    // (which wire this to whatever control a `button { .. on_checked_change:
    // .. }` happens to be attached to), this just toggles a per-control bool
    // on every click.
    windows {
        custom = |_backend, view, callback| {
            use std::cell::Cell;
            use windows_sys::Win32::UI::WindowsAndMessaging::WM_LBUTTONUP;

            let checked = std::rc::Rc::new(Cell::new(false));
            crate::windows::register_raw_hook(view.hwnd, std::rc::Rc::new(move |msg, _wparam, _lparam| {
                if msg == WM_LBUTTONUP {
                    let now = !checked.get();
                    checked.set(now);
                    callback(Event::CheckedChanged(now));
                }
            }));
        }
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

    // `WM_SETFOCUS`/`WM_KILLFOCUS` are sent straight to the control that
    // gained/lost focus, same as android's `OnFocusChangeListener` and the
    // web `focus`/`blur` events above - they only fire for controls that
    // actually receive keyboard focus, which nothing in this backend
    // requests automatically (see `windows/backend.rs`), same caveat as the
    // other two platforms' arms above.
    windows {
        custom = |_backend, view, callback| {
            use windows_sys::Win32::UI::WindowsAndMessaging::{WM_KILLFOCUS, WM_SETFOCUS};

            crate::windows::register_raw_hook(view.hwnd, std::rc::Rc::new(move |msg, _wparam, _lparam| {
                match msg {
                    WM_SETFOCUS => callback(Event::FocusChanged(true)),
                    WM_KILLFOCUS => callback(Event::FocusChanged(false)),
                    _ => {}
                }
            }));
        }
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

    // No editable text control exists in this backend yet (see
    // `windows/state.rs`'s `ControlKind`), so this accumulates `WM_CHAR`
    // straight onto whatever control it's attached to via a per-`HWND`
    // buffer (`windows::append_text_buffer`) - same approximation the web
    // arm above already accepts (`start`/`before` always 0, `count` is the
    // buffer's new length, not the edit size).
    windows {
        custom = |_backend, view, callback| {
            use windows_sys::Win32::UI::WindowsAndMessaging::WM_CHAR;

            let hwnd = view.hwnd;
            crate::windows::register_raw_hook(hwnd, std::rc::Rc::new(move |msg, wparam, _lparam| {
                if msg != WM_CHAR {
                    return;
                }
                let Some(ch) = char::from_u32(wparam as u32) else { return };
                if ch.is_control() {
                    return;
                }
                let text = crate::windows::append_text_buffer(hwnd, ch);
                let count = text.chars().count();
                callback(Event::TextChanged { text, start: 0, before: 0, count });
            }));
        }
    },
}
