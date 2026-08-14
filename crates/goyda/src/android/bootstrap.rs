//! The app's native entry point and its navigation functions
//! ([`navigate`], [`rerender`]).

use std::ffi::c_void;

use goyda_macros::{jcall, jnew, new_widget, set_layout_params, sig, sig_ret};
use jni::objects::{JClass, JObject, JString, JValue};
use jni::strings::JNIString;
use jni::sys::{JNI_FALSE, JNI_TRUE, JNI_VERSION_1_6, jboolean, jint};
use jni::{JNIEnv, JavaVM, NativeMethod};

use crate::android::backend::{AndroidBackend, JVM};
use crate::android::{AndroidBridge, BRIDGE};
use crate::core::theme::{ThemeMode, init_theme_mode};
use crate::find_page;

const UI_MODE_NIGHT_MASK: i32 = 0x30;
const UI_MODE_NIGHT_YES: i32 = 0x20;

fn detect_theme_mode(env: &mut JNIEnv, context: &JObject) -> ThemeMode {
    let resources =
        jcall!(env, context, "getResources", (() -> "android/content/res/Resources"), []);
    let Ok(resources) = resources.and_then(|r| r.l()) else {
        return ThemeMode::Light;
    };

    let configuration = jcall!(env, &resources, "getConfiguration", (() -> "android/content/res/Configuration"), []);
    let Ok(configuration) = configuration.and_then(|r| r.l()) else {
        return ThemeMode::Light;
    };

    let ui_mode = env
        .get_field(&configuration, "uiMode", sig_ret!(int))
        .ok()
        .and_then(|v| v.i().ok())
        .unwrap_or(0);

    if ui_mode & UI_MODE_NIGHT_MASK == UI_MODE_NIGHT_YES {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    }
}

fn wrap_scrollable<'a>(
    env: &mut JNIEnv<'a>,
    context: &JObject<'a>,
    content: JObject<'a>,
) -> JObject<'a> {
    let content_params = jnew!(env, "android/widget/FrameLayout$LayoutParams", ((int, int) -> void), [JValue::Int(-1), JValue::Int(-2)]);
    jcall!(env, &content, "setLayoutParams", (("android/view/ViewGroup$LayoutParams") -> void), [JValue::Object(&content_params)]).unwrap();

    let scroll_view = new_widget!(env, "android/widget/ScrollView", context);
    jcall!(env, &scroll_view, "setFillViewport", ((boolean) -> void), [JValue::Bool(1)]).unwrap();
    jcall!(env, &scroll_view, "addView", (("android/view/View") -> void), [JValue::Object(&content)]).unwrap();
    set_layout_params!(env, &scroll_view, -1, -1);

    scroll_view
}

fn swap_page(env: &mut JNIEnv, bridge: &mut AndroidBridge, page: &crate::Page) {
    let context = env
        .new_local_ref(bridge.context.as_obj())
        .expect("local ref failed");
    let component = (page.factory)();

    let mut android_backend = AndroidBackend::new(env, &context);
    let view = component.render(&mut android_backend);
    let view_jobject = view.as_jobject(env);
    let scrollable = wrap_scrollable(env, &context, view_jobject);

    let root = env
        .new_local_ref(bridge.root.as_obj())
        .expect("local ref failed");
    env.call_method(&root, "removeAllViews", sig!(() -> void), &[])
        .unwrap();
    env.call_method(
        &root,
        "addView",
        sig!(("android/view/View") -> void),
        &[JValue::Object(&scrollable)],
    )
    .unwrap();

    bridge.ui_tree = component;
}

fn native_init(mut env: JNIEnv, _class: JClass, root: JObject) {
    let context = env
        .call_method(
            &root,
            "getContext",
            sig!(() -> "android/content/Context"),
            &[],
        )
        .unwrap()
        .l()
        .unwrap();

    init_theme_mode(detect_theme_mode(&mut env, &context));

    let page = find_page("/").expect("goyda(android): no #[page(\"/\")] registered");
    let component = (page.factory)();

    let mut android_backend = AndroidBackend::new(&mut env, &context);
    let view = component.render(&mut android_backend);

    let view_jobject = view.as_jobject(&mut env);
    let scrollable = wrap_scrollable(&mut env, &context, view_jobject);
    env.call_method(
        &root,
        "addView",
        sig!(("android/view/View") -> void),
        &[JValue::Object(&scrollable)],
    )
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

/// Navigates the app to the `#[page(...)]` registered for `path`.
pub fn navigate(path: &str) {
    let Some(page) = find_page(path) else {
        #[cfg(debug_assertions)]
        eprintln!(
            "goyda(android): navigate(\"{path}\") - no #[page(...)] registered for that route"
        );
        return;
    };

    let Some(jvm) = JVM.get() else { return };
    let Ok(mut env) = jvm.attach_current_thread() else {
        return;
    };
    let Some(bridge_lock) = BRIDGE.get() else {
        return;
    };
    let mut bridge = bridge_lock.lock().unwrap_or_else(|e| e.into_inner());

    swap_page(&mut env, &mut bridge, page);

    let previous_path = std::mem::replace(&mut bridge.current_path, path.to_string());
    bridge.back_stack.push(previous_path);
}

fn native_back(mut env: JNIEnv, _this: JObject) -> jboolean {
    let Some(bridge_lock) = BRIDGE.get() else {
        return JNI_FALSE;
    };
    let mut bridge = bridge_lock.lock().unwrap_or_else(|e| e.into_inner());

    let Some(previous_path) = bridge.back_stack.pop() else {
        return JNI_FALSE;
    };
    let Some(page) = find_page(&previous_path) else {
        return JNI_FALSE;
    };

    swap_page(&mut env, &mut bridge, page);
    bridge.current_path = previous_path;

    JNI_TRUE
}

/// Rebuilds and redisplays the currently mounted page in place, without
/// changing the route.
pub fn rerender() {
    let Some(jvm) = JVM.get() else { return };
    let Ok(mut env) = jvm.attach_current_thread() else {
        return;
    };
    let Some(bridge_lock) = BRIDGE.get() else {
        return;
    };
    let mut bridge = bridge_lock.lock().unwrap_or_else(|e| e.into_inner());

    let Some(page) = find_page(&bridge.current_path) else {
        return;
    };
    swap_page(&mut env, &mut bridge, page);
}

fn native_hot_swap(mut env: JNIEnv, _this: JObject) {
    let Some(bridge_lock) = BRIDGE.get() else {
        return;
    };
    let mut bridge = bridge_lock.lock().unwrap_or_else(|e| e.into_inner());

    let Some(page) = find_page(&bridge.current_path) else {
        return;
    };
    swap_page(&mut env, &mut bridge, page);
}

/// The JNI entry point loaded automatically when the native library is
/// loaded on Android; registers the app's native methods. Not called
/// directly by app code.
#[unsafe(no_mangle)]
pub extern "C" fn JNI_OnLoad(vm: JavaVM, _: *mut c_void) -> jint {
    let vm_ptr = vm.get_java_vm_pointer();

    let _ = JVM.set(vm);

    let reconstructed_vm = unsafe { JavaVM::from_raw(vm_ptr).expect("Cannot reconstruct JavaVM") };
    let mut env = reconstructed_vm
        .get_env()
        .expect("Cannot get JNIEnv from VM");

    let thread_class = env.find_class("java/lang/Thread").unwrap();
    let current_thread = env
        .call_static_method(
            &thread_class,
            "currentThread",
            sig!(() -> "java/lang/Thread"),
            &[],
        )
        .unwrap()
        .l()
        .unwrap();

    let stack_trace_object = env
        .call_method(
            &current_thread,
            "getStackTrace",
            sig!(() -> ["java/lang/StackTraceElement"]),
            &[],
        )
        .unwrap()
        .l()
        .unwrap();

    let stack_trace = jni::objects::JObjectArray::from(stack_trace_object);
    let length = env.get_array_length(&stack_trace).unwrap();

    let mut caller_class_name: Option<String> = None;

    for i in 0..length {
        let element = env.get_object_array_element(&stack_trace, i).unwrap();
        let class_name_object: JString = env
            .call_method(
                &element,
                "getClassName",
                sig!(() -> "java/lang/String"),
                &[],
            )
            .unwrap()
            .l()
            .unwrap()
            .into();
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
        // The stack-walk above finds whichever class happened to trigger
        // this library load - normally `Goyda` (via `new Goyda()` in
        // `MainActivity.start`), but the caller class isn't necessarily the
        // class these `native*` methods are actually *declared* on.
        // Registering onto whatever class happened to trigger the load
        // (rather than `Goyda` specifically) would silently bind them to
        // the wrong class, so this always targets "`Goyda` in the same
        // package as whatever class was found" instead of the found class
        // itself.
        let goyda_class_path = class_path
            .rsplit_once('/')
            .map(|(package, _)| format!("{package}/Goyda"))
            .unwrap_or_else(|| class_path.clone());

        if let Ok(class) = env.find_class(&goyda_class_path) {
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
                NativeMethod {
                    name: JNIString::from("nativeHotSwap"),
                    sig: JNIString::from(sig!(() -> void)),
                    fn_ptr: native_hot_swap as *mut c_void,
                },
            ];
            env.register_native_methods(&class, &methods)
                .expect("Dynamic registration failed");
        }
    }

    JNI_VERSION_1_6
}
