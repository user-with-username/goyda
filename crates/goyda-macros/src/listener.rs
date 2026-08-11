/// Defines a `goyda::platform::active_listeners::on_<name>` module with one
/// implementation per platform, selected at compile time via the `android`
/// / `web` crate features on `goyda`.
///
/// Every invocation must supply *both* an `android { ... }` spec (JNI
/// listener class + native methods, exactly like before) and a `web { ... }`
/// spec (DOM event wiring) - the macro is the only thing that knows how to
/// turn either spec into an `attach(backend, view, callback)` function, so
/// callers only ever provide the event name, the callback type, and
/// per-platform wiring details (which DOM event(s) fire it, which JNI
/// class/methods back it). Both expansions land in a module with the same
/// name (`on_<mod_name>`), each gated behind its own `#[cfg(feature = ...)]`,
/// so `$crate::platform::active_listeners::on_click::attach` resolves
/// regardless of which platform is active.
#[macro_export]
macro_rules! define_listener {
    (
        mod_name = $mod_name:ident,
        callback = $callback_ty:ty,
        android $android_spec:tt,
        web $web_spec:tt $(,)?
    ) => {
        $crate::__define_android_listener! {
            mod_name = $mod_name,
            callback = $callback_ty,
            spec = $android_spec,
        }
        $crate::__define_web_listener! {
            mod_name = $mod_name,
            callback = $callback_ty,
            spec = $web_spec,
        }
    };
}

/// Not public API. The `android { ... }` half of [`define_listener`] -
/// unchanged from the original JNI listener codegen, just split out so the
/// two platform specs can be matched independently.
#[macro_export]
#[doc(hidden)]
macro_rules! __define_android_listener {
    (
        mod_name = $mod_name:ident,
        callback = $callback_ty:ty,
        spec = {
            class_name = $class_name:literal,
            interface_name = $interface_name:literal,
            setter = $setter_method:literal,
            methods = [
                $(
                    {
                        name = $method_name:literal,
                        jni_sig = $jni_sig:tt,
                        native_fn = $native_fn:ident ( $env:ident , $this:ident $(, $arg_name:ident : $arg_ty:ty)* $(,)? ) -> $ret_ty:ty $body:block
                    }
                ),+ $(,)?
            ]
        } $(,)?
    ) => {
        ::paste::paste! {
            #[cfg(feature = "android")]
            pub mod [<on_ $mod_name>] {
                #![allow(dead_code)]

                use ::jni::{JNIEnv, NativeMethod, objects::{JObject, JValue}, sys::jlong};
                use ::jni::strings::JNIString;
                #[allow(unused_imports)]
                use ::jni::sys::{jboolean, jint, JNI_TRUE};
                #[allow(unused_imports)]
                use ::jni::objects::JString;
                use ::std::rc::Rc;
                use ::std::sync::OnceLock;
                #[allow(unused_imports)]
                use crate::core::events::*;

                const CLASS_NAME: &str = $class_name;
                static NATIVE_REGISTERED: OnceLock<()> = OnceLock::new();

                ::inventory::submit! {
                    crate::android::classgen::ListenerSpec {
                        class_name: $class_name,
                        interface_name: $interface_name,
                        methods: &[
                            $(
                                crate::android::classgen::MethodSpec {
                                    name: $method_name,
                                    descriptor: $crate::sig! $jni_sig,
                                }
                            ),+
                        ],
                    }
                }

                pub unsafe fn attach(
                    backend: &mut crate::android::AndroidBackend,
                    view: &crate::android::backend::AndroidView,
                    callback: ::std::rc::Rc<$callback_ty>
                ) {
                    let listener = self::make(backend.env, callback);
                    let local_view = backend.env.new_local_ref(view.global_ref.as_obj()).unwrap();

                    backend.env.call_method(
                        &local_view,
                        $setter_method,
                        concat!("(L", $interface_name, ";)V"),
                        &[::jni::objects::JValue::Object(&listener)],
                    ).unwrap();
                }

                pub fn make<'a>(env: &mut JNIEnv<'a>, callback: Rc<$callback_ty>) -> JObject<'a> {
                    NATIVE_REGISTERED.get_or_init(|| {
                        let class = env
                            .find_class(CLASS_NAME)
                            .expect("no listener to register native-methods");

                        let methods = [
                            $(
                                NativeMethod {
                                    name: JNIString::from($method_name),
                                    sig: JNIString::from($crate::sig! $jni_sig),
                                    fn_ptr: $native_fn as *mut ::std::os::raw::c_void,
                                }
                            ),+
                        ];

                        env.register_native_methods(&class, &methods)
                            .expect("register_native_methods failed");
                    });

                    let boxed: Box<Rc<$callback_ty>> = Box::new(callback);
                    let ptr = Box::into_raw(boxed) as jlong;
                    let class = env.find_class(CLASS_NAME).unwrap();
                    env.new_object(class, "(J)V", &[JValue::Long(ptr)])
                        .expect("no listener obj")
                }

                pub fn get_callback<'a>(env: &mut JNIEnv<'a>, this: &JObject<'a>) -> Option<Rc<$callback_ty>> {
                    let ptr_val = env.get_field(this, "nativePtr", "J").ok()?.j().ok()?;
                    if ptr_val == 0 { return None; }
                    let cb: &Rc<$callback_ty> = unsafe { &*(ptr_val as *const Rc<$callback_ty>) };
                    Some(cb.clone())
                }

                $(
                    #[allow(dead_code)]
                    extern "C" fn $native_fn<'a>(
                        mut $env: ::jni::JNIEnv<'a>,
                        $this: ::jni::objects::JObject<'a>,
                        $($arg_name: $arg_ty),*
                    ) -> $ret_ty {
                        $body
                    }
                )+
            }
        }
    };
}

/// Not public API. The `web { ... }` half of [`define_listener`]. Two forms:
///
/// - `events = [ { dom_event = "...", handler = |evt: EvtTy, cb| { .. } }, ... ]`
///   for listeners that are just "wire N independent DOM events straight to
///   the callback" (click, change, focus/blur, input) - the macro generates
///   the `Closure`, `add_event_listener_with_callback`, and `.forget()` for
///   each one.
/// - `custom = |backend, view, callback| { .. }` as an escape hatch for
///   listeners whose DOM wiring needs shared state across multiple events
///   (e.g. a press-and-hold timer for long-click) that doesn't fit the
///   independent-events shape.
#[macro_export]
#[doc(hidden)]
macro_rules! __define_web_listener {
    (
        mod_name = $mod_name:ident,
        callback = $callback_ty:ty,
        spec = {
            events = [
                $(
                    {
                        dom_event = $dom_event:literal,
                        handler = |$evt_ident:ident : $evt_ty:ty, $cb_ident:ident| $handler_body:block
                    }
                ),+ $(,)?
            ]
        } $(,)?
    ) => {
        ::paste::paste! {
            #[cfg(feature = "web")]
            pub mod [<on_ $mod_name>] {
                #![allow(dead_code)]

                use ::std::rc::Rc;
                use ::wasm_bindgen::closure::Closure;
                use ::wasm_bindgen::JsCast;
                #[allow(unused_imports)]
                use crate::core::events::*;
                use crate::web::backend::{WebBackend, WebView};

                pub unsafe fn attach(_backend: &mut WebBackend, view: &WebView, callback: Rc<$callback_ty>) {
                    $(
                        {
                            let $cb_ident: Rc<$callback_ty> = callback.clone();
                            let closure = Closure::<dyn FnMut($evt_ty)>::new(move |$evt_ident: $evt_ty| $handler_body);
                            let _ = view
                                .element
                                .add_event_listener_with_callback($dom_event, closure.as_ref().unchecked_ref());
                            closure.forget();
                        }
                    )+
                }
            }
        }
    };

    (
        mod_name = $mod_name:ident,
        callback = $callback_ty:ty,
        spec = {
            custom = |$backend_ident:ident, $view_ident:ident, $callback_ident:ident| $custom_body:block
        } $(,)?
    ) => {
        ::paste::paste! {
            #[cfg(feature = "web")]
            pub mod [<on_ $mod_name>] {
                #![allow(dead_code)]

                #[allow(unused_imports)]
                use crate::core::events::*;
                use crate::web::backend::{WebBackend, WebView};

                pub unsafe fn attach(
                    $backend_ident: &mut WebBackend,
                    $view_ident: &WebView,
                    $callback_ident: ::std::rc::Rc<$callback_ty>,
                ) $custom_body
            }
        }
    };
}
