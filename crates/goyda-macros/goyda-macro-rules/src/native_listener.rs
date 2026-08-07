#[macro_export]
macro_rules! define_native_listener {
    (
        mod_name = $mod_name:ident,
        class_name = $class_name:literal,
        interface_name = $interface_name:literal,
        setter = $setter_method:literal,
        callback = $callback_ty:ty,
        methods = [
            $(
                {
                    name = $method_name:literal,
                    jni_sig = $jni_sig:tt,
                    native_fn = $native_fn:ident ( $env:ident , $this:ident $(, $arg_name:ident : $arg_ty:ty)* $(,)? ) -> $ret_ty:ty $body:block
                }
            ),+ $(,)?
        ]
    ) => {
        ::paste::paste! {
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
                    crate::android::ListenerSpec {
                        class_name: $class_name,
                        interface_name: $interface_name,
                        methods: &[
                            $(
                                crate::android::MethodSpec {
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
