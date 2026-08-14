/// Builds a JNI method signature string from a `(args) -> ret` shape.
///
/// Argument and return types are Java primitive keywords (`int`, `boolean`,
/// `float`, `long`, `byte`, `void`), an array (`[type]`), or a fully
/// qualified class name as a string literal.
///
/// ```ignore
/// let d = sig!((int, "java/lang/String") -> boolean);
/// assert_eq!(d, "(ILjava/lang/String;)Z");
/// ```
#[macro_export]
macro_rules! sig {
    ( ($($args:tt)*) -> $ret:tt ) => {
        concat!(
            "(",
            $crate::sig_args!($($args)*),
            ")",
            $crate::sig_ret!($ret)
        )
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! sig_args {
    () => { "" };

    ([$elem:tt], $($tail:tt)*) => { concat!("[", $crate::sig_ret!($elem), $crate::sig_args!($($tail)*)) };
    (int, $($tail:tt)*) => { concat!("I", $crate::sig_args!($($tail)*)) };
    (boolean, $($tail:tt)*) => { concat!("Z", $crate::sig_args!($($tail)*)) };
    (float, $($tail:tt)*) => { concat!("F", $crate::sig_args!($($tail)*)) };
    (long, $($tail:tt)*) => { concat!("J", $crate::sig_args!($($tail)*)) };
    (byte, $($tail:tt)*) => { concat!("B", $crate::sig_args!($($tail)*)) };
    (void, $($tail:tt)*) => { concat!("V", $crate::sig_args!($($tail)*)) };
    ($class:literal, $($tail:tt)*) => { concat!("L", $class, ";", $crate::sig_args!($($tail)*)) };

    ([$elem:tt] $($tail:tt)*) => { concat!("[", $crate::sig_ret!($elem), $crate::sig_args!($($tail)*)) };
    (int $($tail:tt)*) => { concat!("I", $crate::sig_args!($($tail)*)) };
    (boolean $($tail:tt)*) => { concat!("Z", $crate::sig_args!($($tail)*)) };
    (float $($tail:tt)*) => { concat!("F", $crate::sig_args!($($tail)*)) };
    (long $($tail:tt)*) => { concat!("J", $crate::sig_args!($($tail)*)) };
    (byte $($tail:tt)*) => { concat!("B", $crate::sig_args!($($tail)*)) };
    (void $($tail:tt)*) => { concat!("V", $crate::sig_args!($($tail)*)) };
    ($class:literal $($tail:tt)*) => { concat!("L", $class, ";", $crate::sig_args!($($tail)*)) };
}

#[macro_export]
#[doc(hidden)]
macro_rules! sig_ret {
    (int) => {
        "I"
    };
    (boolean) => {
        "Z"
    };
    (float) => {
        "F"
    };
    (long) => {
        "J"
    };
    (byte) => {
        "B"
    };
    (void) => {
        "V"
    };
    ([$elem:tt]) => {
        concat!("[", $crate::sig_ret!($elem))
    };
    ($class:literal) => {
        concat!("L", $class, ";")
    };
}
