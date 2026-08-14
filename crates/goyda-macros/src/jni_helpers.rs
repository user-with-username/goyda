//! JNI helper macros for calling and constructing Java objects with a
//! compile-time-checked method signature.

/// Calls a JVM instance method, building its signature from a `(args) -> ret` shape.
///
/// ```ignore
/// jcall!(env, &view, "setEnabled", ((boolean) -> void), [JValue::Bool(1)])?;
/// ```
#[macro_export]
macro_rules! jcall {
    ($env:expr, $target:expr, $name:literal, $sig:tt, [$($arg:expr),* $(,)?]) => {
        $env.call_method($target, $name, $crate::sig! $sig, &[$($arg),*])
    };
}

/// Constructs a JVM object, building its constructor signature from a `(args) -> ret` shape.
///
/// ```ignore
/// let obj = jnew!(env, "android/widget/TextView", (("android/content/Context") -> void), [JValue::Object(&ctx)]);
/// ```
#[macro_export]
macro_rules! jnew {
    ($env:expr, $class:expr, $sig:tt, [$($arg:expr),* $(,)?]) => {
        $env.new_object($class, $crate::sig! $sig, &[$($arg),*]).unwrap()
    };
}

/// Constructs a widget class by calling its single-`Context` constructor.
///
/// ```ignore
/// let view = new_widget!(env, "android/widget/TextView", &ctx);
/// ```
#[macro_export]
macro_rules! new_widget {
    ($env:expr, $class:expr, $context:expr) => {
        $crate::jnew!($env, $class, (("android/content/Context") -> void), [::jni::objects::JValue::Object($context)])
    };
}

/// Builds a `width` x `height` layout params object and applies it to a view.
///
/// ```ignore
/// set_layout_params!(env, &view, 200, 100);
/// ```
#[macro_export]
macro_rules! set_layout_params {
    ($env:expr, $view:expr, $w:expr, $h:expr) => {{
        let params = $crate::jnew!(
            $env,
            "android/widget/LinearLayout$LayoutParams",
            ((int, int) -> void),
            [::jni::objects::JValue::Int($w), ::jni::objects::JValue::Int($h)]
        );
        $crate::jcall!(
            $env,
            $view,
            "setLayoutParams",
            (("android/view/ViewGroup$LayoutParams") -> void),
            [::jni::objects::JValue::Object(&params)]
        ).unwrap();
    }};
}
