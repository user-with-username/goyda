pub mod backend;
pub use backend::AndroidBackend;
pub mod bootstrap;
pub mod classgen;

pub use bootstrap::navigate;

use jni::objects::GlobalRef;
use once_cell::sync::OnceCell;
use std::sync::Mutex;

pub static BRIDGE: OnceCell<Mutex<AndroidBridge>> = OnceCell::new();

pub struct AndroidBridge {
    pub root: GlobalRef,
    pub context: GlobalRef,
    pub ui_tree: crate::Component,
    pub current_path: String,
    pub back_stack: Vec<String>,
}

/// # Safety
///
/// Safe because `AndroidBridge` is exclusively accessed from the single-threaded Android Main UI Thread.
unsafe impl Send for AndroidBridge {}
