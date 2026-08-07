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

    (int, $($tail:tt)*) => { concat!("I", $crate::sig_args!($($tail)*)) };
    (float, $($tail:tt)*) => { concat!("F", $crate::sig_args!($($tail)*)) };
    (void, $($tail:tt)*) => { concat!("V", $crate::sig_args!($($tail)*)) };
    ($class:literal, $($tail:tt)*) => { concat!("L", $class, ";", $crate::sig_args!($($tail)*)) };

    (int $($tail:tt)*) => { concat!("I", $crate::sig_args!($($tail)*)) };
    (float $($tail:tt)*) => { concat!("F", $crate::sig_args!($($tail)*)) };
    (void $($tail:tt)*) => { concat!("V", $crate::sig_args!($($tail)*)) };
    ($class:literal $($tail:tt)*) => { concat!("L", $class, ";", $crate::sig_args!($($tail)*)) };
}

#[macro_export]
#[doc(hidden)]
macro_rules! sig_ret {
    (int) => { "I" };
    (float) => { "F" };
    (void) => { "V" };
    ($class:literal) => { concat!("L", $class, ";") };
}
