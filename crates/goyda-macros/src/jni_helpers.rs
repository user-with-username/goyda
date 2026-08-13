//! Small `sig!`-backed wrappers for the JNI call shapes that show up over
//! and over in `goyda::android::backend` - a plain-`Context` widget
//! constructor, a `call_method`/`new_object` whose descriptor is built by
//! hand today, and the `LinearLayout$LayoutParams(w, h)` + `setLayoutParams`
//! pair every `create_*` method ends with. Hand-typing JNI descriptors is
//! exactly how the `onCheckedChanged(...I)V` vs. the real
//! `onCheckedChanged(...Z)V` bug happened (see `goyda::listeners`) - routing
//! every descriptor through `sig!` instead means a typo becomes a compile
//! error in the macro (unknown token) rather than a silent runtime
//! `AbstractMethodError`/`NoSuchMethodError` on-device.

/// `env.call_method($target, $name, sig!($sig), &[$($arg),*])` - the
/// `$sig` tuple is the same `(args) -> ret` shape `sig!` and
/// `define_listener!`'s `jni_sig` already use, so a call and a listener
/// method share one syntax for "what does the JVM see here".
#[macro_export]
macro_rules! jcall {
    ($env:expr, $target:expr, $name:literal, $sig:tt, [$($arg:expr),* $(,)?]) => {
        $env.call_method($target, $name, $crate::sig! $sig, &[$($arg),*])
    };
}

/// `env.new_object($class, sig!($sig), &[$($arg),*])`, unwrapped - every
/// call site here already treats construction failure as fatal (`.unwrap()`
/// on `new_object` throughout `backend.rs`), so this saves repeating that.
#[macro_export]
macro_rules! jnew {
    ($env:expr, $class:expr, $sig:tt, [$($arg:expr),* $(,)?]) => {
        $env.new_object($class, $crate::sig! $sig, &[$($arg),*]).unwrap()
    };
}

/// `new Widget(context)` - every `create_*` in `AndroidBackend` starts by
/// constructing some `android/widget/*` (or `android/view/View`) with the
/// single-`Context` constructor; this is that, minus the descriptor
/// boilerplate. `$class` takes any class-name expression (a literal, or a
/// runtime-chosen one like `create_scroll_view`'s horizontal/vertical
/// pick), not just a literal.
#[macro_export]
macro_rules! new_widget {
    ($env:expr, $class:expr, $context:expr) => {
        $crate::jnew!($env, $class, (("android/content/Context") -> void), [::jni::objects::JValue::Object($context)])
    };
}

/// Builds a `LinearLayout$LayoutParams(width, height)` and immediately
/// applies it via `setLayoutParams` - the pair every `create_*` method
/// finishes with, differing only in the two `i32`s.
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
