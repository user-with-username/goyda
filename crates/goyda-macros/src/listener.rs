/// Declares an event listener that works across every supported platform.
///
/// Give it a name, the callback signature, and how to wire the event up on
/// Android, web, and Windows; it produces a matching `on_<name>` module
/// whose `attach` function you can call from any platform build.
///
/// ```ignore
/// goyda_macros::define_listener! {
///     mod_name = click,
///     callback = dyn Fn(),
///     android { /* JNI listener class + methods */ },
///     web { events = [ /* DOM events */ ] },
///     windows { custom = |backend, view, callback| { /* WM_* wiring */ } },
/// }
/// ```
#[macro_export]
macro_rules! define_listener {
    (
        mod_name = $mod_name:ident,
        callback = $callback_ty:ty,
        android $android_spec:tt,
        web $web_spec:tt,
        windows $windows_spec:tt $(,)?
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
        $crate::__define_windows_listener! {
            mod_name = $mod_name,
            callback = $callback_ty,
            spec = $windows_spec,
        }
    };
}

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
                        $crate::sig!(($interface_name) -> void),
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
                    env.new_object(class, $crate::sig!((long) -> void), &[JValue::Long(ptr)])
                        .expect("no listener obj")
                }

                pub fn get_callback<'a>(env: &mut JNIEnv<'a>, this: &JObject<'a>) -> Option<Rc<$callback_ty>> {
                    let ptr_val = env.get_field(this, "nativePtr", $crate::sig_ret!(long)).ok()?.j().ok()?;
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

#[macro_export]
#[doc(hidden)]
macro_rules! __define_windows_listener {
    (
        mod_name = $mod_name:ident,
        callback = $callback_ty:ty,
        spec = {
            custom = |$backend_ident:ident, $view_ident:ident, $callback_ident:ident| $custom_body:block
        } $(,)?
    ) => {
        ::paste::paste! {
            #[cfg(feature = "windows")]
            pub mod [<on_ $mod_name>] {
                #![allow(dead_code)]

                #[allow(unused_imports)]
                use crate::core::events::*;
                use crate::windows::backend::{WindowsBackend, WindowsView};

                pub unsafe fn attach(
                    $backend_ident: &mut WindowsBackend,
                    $view_ident: &WindowsView,
                    $callback_ident: ::std::rc::Rc<$callback_ty>,
                ) $custom_body
            }
        }
    };
}
