//! The app's single native entry point. Unlike the web target (where
//! `#[wasm_bindgen(start)]` can be attached once per `#[page(...)]` function
//! harmlessly, since only the module the browser actually loads matters),
//! Android needs exactly one `JNI_OnLoad`/`nativeInit` pair per app - two
//! `#[page(...)]` functions each emitting their own would collide at link
//! time. So this lives here, built once regardless of how many pages a
//! consumer crate registers, and picks the initial page to render from the
//! `Page` inventory (see [`crate::find_page`]) instead of hardcoding one.

use std::ffi::c_void;

use goyda_macros::{jcall, jnew, new_widget, set_layout_params, sig, sig_ret};
use jni::objects::{JClass, JObject, JString, JValue};
use jni::strings::JNIString;
use jni::sys::{jboolean, jint, JNI_FALSE, JNI_TRUE, JNI_VERSION_1_6};
use jni::{JNIEnv, JavaVM, NativeMethod};

use crate::android::backend::{AndroidBackend, JVM};
use crate::android::{AndroidBridge, BRIDGE};
use crate::core::theme::{init_theme_mode, ThemeMode};
use crate::find_page;

/// `Configuration.UI_MODE_NIGHT_MASK`/`UI_MODE_NIGHT_YES` - stable,
/// long-frozen platform constants, so hardcoding them here is simpler than
/// a JNI static-field lookup for values that never change.
const UI_MODE_NIGHT_MASK: i32 = 0x30;
const UI_MODE_NIGHT_YES: i32 = 0x20;

/// Reads `Context.getResources().getConfiguration().uiMode` to seed the
/// initial [`ThemeMode`] from whatever the device (or the app's own
/// requested night-mode override) is actually set to - see
/// `crate::core::theme`'s doc comment for how this plugs into `theme!`.
fn detect_theme_mode(env: &mut JNIEnv, context: &JObject) -> ThemeMode {
    let resources = jcall!(env, context, "getResources", (() -> "android/content/res/Resources"), []);
    let Ok(resources) = resources.and_then(|r| r.l()) else { return ThemeMode::Light };

    let configuration = jcall!(env, &resources, "getConfiguration", (() -> "android/content/res/Configuration"), []);
    let Ok(configuration) = configuration.and_then(|r| r.l()) else { return ThemeMode::Light };

    let ui_mode = env.get_field(&configuration, "uiMode", sig_ret!(int)).ok().and_then(|v| v.i().ok()).unwrap_or(0);

    if ui_mode & UI_MODE_NIGHT_MASK == UI_MODE_NIGHT_YES {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    }
}

/// Wraps `content` (a page's already-rendered root `View`) in a vertically
/// scrolling `ScrollView`, so a page taller than the screen scrolls instead
/// of silently clipping at the bottom edge - the built-in
/// `Component::scroll_view` isn't reused here since its children loop sizes
/// each child `WRAP_CONTENT` and center-gravity (meant for a scrollable
/// *list* of independently-sized items), which would shrink a full-width
/// page down to its narrowest natural width instead of keeping it filling
/// the screen; `content` keeps `MATCH_PARENT` width here and only trades
/// `MATCH_PARENT` height for `WRAP_CONTENT`, since only *that* lets it grow
/// past the viewport for the `ScrollView` to scroll over.
///
/// `WRAP_CONTENT` cuts both ways though: a page *shorter* than the screen
/// now only paints its own background over its own (short) height, leaving
/// the `ScrollView`'s unstyled remainder showing through as a plain black/
/// white band underneath - `setFillViewport(true)` is the standard fix,
/// stretching `content` to fill the viewport whenever its natural height
/// is the smaller one, while still leaving it free to grow past the
/// viewport (and scroll) whenever its natural height is the bigger one.
fn wrap_scrollable<'a>(env: &mut JNIEnv<'a>, context: &JObject<'a>, content: JObject<'a>) -> JObject<'a> {
    let content_params = jnew!(env, "android/widget/FrameLayout$LayoutParams", ((int, int) -> void), [JValue::Int(-1), JValue::Int(-2)]);
    jcall!(env, &content, "setLayoutParams", (("android/view/ViewGroup$LayoutParams") -> void), [JValue::Object(&content_params)]).unwrap();

    let scroll_view = new_widget!(env, "android/widget/ScrollView", context);
    jcall!(env, &scroll_view, "setFillViewport", ((boolean) -> void), [JValue::Bool(1)]).unwrap();
    jcall!(env, &scroll_view, "addView", (("android/view/View") -> void), [JValue::Object(&content)]).unwrap();
    set_layout_params!(env, &scroll_view, -1, -1);

    scroll_view
}

/// Renders `page` into `bridge`'s root, replacing whatever was mounted
/// before - the shared body of [`navigate`], [`native_back`], and
/// [`rerender`], which differ only in how they get to "here's the page to
/// show" (a new route, a popped back-stack entry, or the same route again
/// after a `theme!` change).
fn swap_page(env: &mut JNIEnv, bridge: &mut AndroidBridge, page: &crate::Page) {
    let context = env.new_local_ref(bridge.context.as_obj()).expect("local ref failed");
    let component = (page.factory)();

    let mut android_backend = AndroidBackend::new(env, &context);
    let view = component.render(&mut android_backend);
    let view_jobject = view.as_jobject(env);
    let scrollable = wrap_scrollable(env, &context, view_jobject);

    let root = env.new_local_ref(bridge.root.as_obj()).expect("local ref failed");
    env.call_method(&root, "removeAllViews", sig!(() -> void), &[]).unwrap();
    env.call_method(&root, "addView", sig!(("android/view/View") -> void), &[JValue::Object(&scrollable)]).unwrap();

    bridge.ui_tree = component;
}

fn native_init(mut env: JNIEnv, _class: JClass, root: JObject) {
    let context = env
        .call_method(&root, "getContext", sig!(() -> "android/content/Context"), &[])
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
    env.call_method(&root, "addView", sig!(("android/view/View") -> void), &[JValue::Object(&scrollable)])
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

    swap_page(&mut env, &mut bridge, page);

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

    swap_page(&mut env, &mut bridge, page);
    bridge.current_path = previous_path;

    JNI_TRUE
}

/// Re-renders whichever page is currently mounted, in place - no route
/// change, no back-stack entry. Used by [`crate::core::theme::set_theme_mode`]
/// so a runtime `theme!` switch shows up immediately.
pub fn rerender() {
    let Some(jvm) = JVM.get() else { return };
    let Ok(mut env) = jvm.attach_current_thread() else { return };
    let Some(bridge_lock) = BRIDGE.get() else { return };
    let mut bridge = bridge_lock.lock().unwrap_or_else(|e| e.into_inner());

    let Some(page) = find_page(&bridge.current_path) else { return };
    swap_page(&mut env, &mut bridge, page);
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
