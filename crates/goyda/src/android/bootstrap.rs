//! The app's single native entry point. Unlike the web target (where
//! `#[wasm_bindgen(start)]` can be attached once per `#[page(...)]` function
//! harmlessly, since only the module the browser actually loads matters),
//! Android needs exactly one `JNI_OnLoad`/`nativeInit` pair per app - two
//! `#[page(...)]` functions each emitting their own would collide at link
//! time. So this lives here, built once regardless of how many pages a
//! consumer crate registers, and picks the initial page to render from the
//! `Page` inventory (see [`crate::find_page`]) instead of hardcoding one.

use std::ffi::c_void;

use goyda_macros::sig;
use jni::objects::{JClass, JObject, JString, JValue};
use jni::strings::JNIString;
use jni::sys::{jboolean, jint, JNI_FALSE, JNI_TRUE, JNI_VERSION_1_6};
use jni::{JNIEnv, JavaVM, NativeMethod};

use crate::android::backend::{AndroidBackend, JVM};
use crate::android::{AndroidBridge, BRIDGE};
use crate::find_page;

fn native_init(mut env: JNIEnv, _class: JClass, root: JObject) {
    let context = env
        .call_method(&root, "getContext", sig!(() -> "android/content/Context"), &[])
        .unwrap()
        .l()
        .unwrap();

    let page = find_page("/").expect("goyda(android): no #[page(\"/\")] registered");
    let component = (page.factory)();

    let mut android_backend = AndroidBackend::new(&mut env, &context);
    let view = component.render(&mut android_backend);

    let view_jobject = view.as_jobject(&mut env);
    env.call_method(&root, "addView", sig!(("android/view/View") -> void), &[JValue::Object(&view_jobject)])
        .unwrap();

    let global_root = env.new_global_ref(&root).expect("GlobalRef failed");
    let global_context = env.new_global_ref(&context).expect("GlobalRef failed");

    let bridge = AndroidBridge {
        root: global_root,
        context: global_context,
        ui_tree: component,
        current_path: "/".to_string(),
        back_stack: Vec::new(),
    };
    let _ = BRIDGE.set(std::sync::Mutex::new(bridge));
}

/// Switches the mounted app to whichever `#[page(...)]` is registered for
/// `path` (see [`crate::find_page`]), replacing the root `ViewGroup`'s
/// children with the newly rendered page.
pub fn navigate(path: &str) {
    let Some(page) = find_page(path) else {
        #[cfg(debug_assertions)]
        eprintln!("goyda(android): navigate(\"{path}\") - no #[page(...)] registered for that route");
        return;
    };

    let Some(jvm) = JVM.get() else { return };
    let Ok(mut env) = jvm.attach_current_thread() else { return };
    let Some(bridge_lock) = BRIDGE.get() else { return };
    let mut bridge = bridge_lock.lock().unwrap_or_else(|e| e.into_inner());

    let context = env.new_local_ref(bridge.context.as_obj()).expect("local ref failed");
    let component = (page.factory)();

    let mut android_backend = AndroidBackend::new(&mut env, &context);
    let view = component.render(&mut android_backend);
    let view_jobject = view.as_jobject(&mut env);

    let root = env.new_local_ref(bridge.root.as_obj()).expect("local ref failed");
    env.call_method(&root, "removeAllViews", sig!(() -> void), &[]).unwrap();
    env.call_method(&root, "addView", sig!(("android/view/View") -> void), &[JValue::Object(&view_jobject)]).unwrap();

    bridge.ui_tree = component;
    let previous_path = std::mem::replace(&mut bridge.current_path, path.to_string());
    bridge.back_stack.push(previous_path);
}

/// Pops the in-app back stack built up by [`navigate`] and re-renders
/// whichever page was on top of it, the same way the browser's back button
/// re-renders on `popstate` for the web target (see `crate::web::mod`) -
/// Android has no OS-level page stack of its own to lean on (single
/// `Activity`, no fragment back stack), so `MainActivity`'s `onBackPressed`
/// calls this via `nativeBack` instead. Returns `false` when the stack is
/// empty so the caller falls back to the platform default (finishing the
/// `Activity`).
fn native_back(mut env: JNIEnv, _this: JObject) -> jboolean {
    let Some(bridge_lock) = BRIDGE.get() else { return JNI_FALSE };
    let mut bridge = bridge_lock.lock().unwrap_or_else(|e| e.into_inner());

    let Some(previous_path) = bridge.back_stack.pop() else { return JNI_FALSE };

    let Some(page) = find_page(&previous_path) else { return JNI_FALSE };

    let context = env.new_local_ref(bridge.context.as_obj()).expect("local ref failed");
    let component = (page.factory)();

    let mut android_backend = AndroidBackend::new(&mut env, &context);
    let view = component.render(&mut android_backend);
    let view_jobject = view.as_jobject(&mut env);

    let root = env.new_local_ref(bridge.root.as_obj()).expect("local ref failed");
    env.call_method(&root, "removeAllViews", sig!(() -> void), &[]).unwrap();
    env.call_method(&root, "addView", sig!(("android/view/View") -> void), &[JValue::Object(&view_jobject)]).unwrap();

    bridge.ui_tree = component;
    bridge.current_path = previous_path;

    JNI_TRUE
}

/// Finds the Java/Kotlin class that called into native code (by walking the
/// current thread's stack trace and skipping framework frames) and
/// dynamically registers `native_init` on it as `nativeInit`. Avoids
/// requiring consumer apps to know or declare the native method signature
/// themselves - `Goyda.java`'s `nativeInit(ViewGroup)` call is all that's
/// needed on the Java side.
#[unsafe(no_mangle)]
pub extern "C" fn JNI_OnLoad(vm: JavaVM, _: *mut c_void) -> jint {
    let vm_ptr = vm.get_java_vm_pointer();

    let _ = JVM.set(vm);

    let reconstructed_vm = unsafe { JavaVM::from_raw(vm_ptr).expect("Cannot reconstruct JavaVM") };
    let mut env = reconstructed_vm.get_env().expect("Cannot get JNIEnv from VM");

    let thread_class = env.find_class("java/lang/Thread").unwrap();
    let current_thread = env
        .call_static_method(&thread_class, "currentThread", sig!(() -> "java/lang/Thread"), &[])
        .unwrap()
        .l()
        .unwrap();

    let stack_trace_object = env
        .call_method(&current_thread, "getStackTrace", sig!(() -> ["java/lang/StackTraceElement"]), &[])
        .unwrap()
        .l()
        .unwrap();

    let stack_trace = jni::objects::JObjectArray::from(stack_trace_object);
    let length = env.get_array_length(&stack_trace).unwrap();

    let mut caller_class_name: Option<String> = None;

    for i in 0..length {
        let element = env.get_object_array_element(&stack_trace, i).unwrap();
        let class_name_object: JString =
            env.call_method(&element, "getClassName", sig!(() -> "java/lang/String"), &[]).unwrap().l().unwrap().into();
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
            let methods = [
                NativeMethod {
                    name: JNIString::from("nativeInit"),
                    sig: JNIString::from(sig!(("android/view/ViewGroup") -> void)),
                    fn_ptr: native_init as *mut c_void,
                },
                NativeMethod {
                    name: JNIString::from("nativeBack"),
                    sig: JNIString::from(sig!(() -> boolean)),
                    fn_ptr: native_back as *mut c_void,
                },
            ];
            env.register_native_methods(&class, &methods).expect("Dynamic registration failed");
        }
    }

    JNI_VERSION_1_6
}
