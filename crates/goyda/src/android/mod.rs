pub mod backend;
pub use backend::AndroidBackend;
pub mod bootstrap;
pub mod classgen;

pub use bootstrap::{navigate, rerender};

use jni::objects::GlobalRef;
use once_cell::sync::OnceCell;
use std::sync::Mutex;

/// The running app's connection to its Android host, set once at startup.
pub static BRIDGE: OnceCell<Mutex<AndroidBridge>> = OnceCell::new();

/// State shared between `goyda` and the Android host it's embedded in.
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
