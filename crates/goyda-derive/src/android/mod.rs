use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub fn jni_code(fn_name: &Ident) -> TokenStream {
    quote! {
        fn native_init(mut env: JNIEnv, _class: JClass, root: JObject) {
            let context = env
                .call_method(
                    &root, 
                    "getContext", 
                    "()Landroid/content/Context;", 
                    &[]
                )
                .unwrap()
                .l()
                .unwrap();

            let ir_ui = crate::#fn_name();

            let mut android_backend = AndroidBackend::new(&mut env, &context);

            let goyda_android_view = ir_ui.render(&mut android_backend);

            let view_jobject = goyda_android_view.as_jobject(&mut env);
            
            env.call_method(
                &root,
                "addView",
                "(Landroid/view/View;)V",
                &[JValue::Object(&view_jobject)],
            ).unwrap();

            let global_root = env.new_global_ref(root).expect("GlobalRef failed");

            let bridge = AndroidBridge {
                root: global_root,
                ui_tree: ir_ui,
            };
            
            let _ = BRIDGE.set(std::sync::Mutex::new(bridge));
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn JNI_OnLoad(vm: JavaVM, _: *mut c_void) -> jint {
            let vm_ptr = vm.get_java_vm_pointer();

            let _ = ::goyda::android::backend::JVM.set(vm);

            let reconstructed_vm = unsafe { 
                ::goyda::jni::JavaVM::from_raw(vm_ptr)
                    .expect("Cannot reconstruct JavaVM")
            };
            
            let mut env = reconstructed_vm.get_env().expect("Cannot get JNIEnv from VM");

            let thread_class = env.find_class("java/lang/Thread").unwrap();
            let current_thread = env.call_static_method(
                &thread_class,
                "currentThread",
                "()Ljava/lang/Thread;",
                &[]
            ).unwrap().l().unwrap();

            let stack_trace_object = env.call_method(
                &current_thread,
                "getStackTrace",
                "()[Ljava/lang/StackTraceElement;",
                &[]
            ).unwrap().l().unwrap();

            let stack_trace = ::goyda::jni::objects::JObjectArray::from(stack_trace_object);
            let length = env.get_array_length(&stack_trace).unwrap();

            let mut caller_class_name: Option<String> = None;

            for i in 0..length {
                let element = env.get_object_array_element(&stack_trace, i).unwrap();
                let class_name_object: JString = env.call_method(
                    &element,
                    "getClassName",
                    "()Ljava/lang/String;",
                    &[]
                ).unwrap().l().unwrap().into();
                let class_str: String = env.get_string(&class_name_object).unwrap().into();

                if !class_str.starts_with("java.")
                    && !class_str.starts_with("android.")
                    && !class_str.starts_with("dalvik.")
                    && class_str != "com.android.internal.os.RuntimeInit"
                {
                    caller_class_name = Some(class_str.replace('.', "/"));
                    break;
                }
            }

            if let Some(class_path) = caller_class_name {
                if let Ok(class) = env.find_class(&class_path) {
                    let method = NativeMethod {
                        name: JNIString::from("nativeInit"),
                        sig: JNIString::from("(Landroid/view/ViewGroup;)V"),
                        fn_ptr: native_init as *mut c_void,
                    };
                    env.register_native_methods(&class, &[method])
                        .expect("Dynamic registration failed");
                }
            }

            JNI_VERSION_1_6
        }
    }
}
