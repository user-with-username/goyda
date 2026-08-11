pub mod backend;
pub use backend::AndroidBackend;
pub mod classgen;

use jni::objects::GlobalRef;
use once_cell::sync::OnceCell;
use std::sync::Mutex;

pub static BRIDGE: OnceCell<Mutex<AndroidBridge>> = OnceCell::new();

pub struct AndroidBridge {
    pub root: GlobalRef,
    pub ui_tree: crate::Component, 
}

/// # Safety
///
/// Safe because `AndroidBridge` is exclusively accessed from the single-threaded Android Main UI Thread.
unsafe impl Send for AndroidBridge {}