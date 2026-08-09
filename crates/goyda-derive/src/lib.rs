extern crate proc_macro;

mod android;
mod utils;
mod scanner;
mod transformer;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, LitStr};
use syn::visit_mut::VisitMut;
use transformer::{ReactivityGraphTransformer, UiMacroTransformer};

#[proc_macro_attribute]
pub fn page(attr: TokenStream, item: TokenStream) -> TokenStream {
    let route_path = parse_macro_input!(attr as LitStr);
    let mut input_fn = parse_macro_input!(item as ItemFn);

    let mut graph_transformer = ReactivityGraphTransformer::new();
    graph_transformer.build_dependency_graph(&input_fn);
    
    let mut_vars_cache = graph_transformer.all_mut_vars.clone();
    graph_transformer.visit_item_fn_mut(&mut input_fn);

    let mut ui_transformer = UiMacroTransformer { all_mut_vars: mut_vars_cache };
    ui_transformer.visit_item_fn_mut(&mut input_fn);

    let fn_name = &input_fn.sig.ident;
    let module_name = syn::Ident::new(&format!("__goyda_page_container_{}", fn_name), fn_name.span());
    let register_name = syn::Ident::new(&format!("__GOYDA_PAGE_{}", fn_name), fn_name.span());
    let jni_code = android::jni_code(fn_name);

    let expanded = quote! {
        #input_fn

        #[doc(hidden)]
        mod #module_name {
            use super::*;

            #[allow(non_upper_case_globals)]
            pub static #register_name: ::goyda::Page = ::goyda::Page::new(
                #route_path,
                || crate::#fn_name()
            );

            ::goyda::inventory::submit! {
                #register_name
            }

            #[cfg(target_os = "android")]
            #[doc(hidden)]
            mod __goyda_backend {
                use ::goyda::jni::{
                    objects::{JClass, JObject, JString, JValue, JMethodID},
                    sys::{jint, jlong, JNI_VERSION_1_6},
                    JNIEnv, JavaVM, NativeMethod,
                };
                use ::goyda::jni::strings::JNIString;
                use ::goyda::android::backend::{AndroidBackend, AndroidView, JVM};
                use ::std::ffi::c_void;
                use ::std::cell::RefCell;

                type AndroidBridge = ::goyda::android::AndroidBridge;

                thread_local! {
                    pub static REAL_BRIDGE: RefCell<Option<AndroidBridge>> = RefCell::new(None);
                }

                #[allow(non_camel_case_types)]
                pub struct BRIDGE_EMULATOR;

                pub trait UnwrappableBridge {
                    fn unwrap_bridge(self) -> AndroidBridge;
                }

                impl UnwrappableBridge for AndroidBridge {
                    fn unwrap_bridge(self) -> AndroidBridge { self }
                }

                impl UnwrappableBridge for ::std::sync::Mutex<AndroidBridge> {
                    fn unwrap_bridge(self) -> AndroidBridge {
                        self.into_inner().unwrap_or_else(|e| e.into_inner())
                    }
                }

                impl BRIDGE_EMULATOR {
                    pub fn set<U: UnwrappableBridge>(&self, value: U) -> Result<(), &'static str> {
                        REAL_BRIDGE.with(|cell| {
                            *cell.borrow_mut() = Some(value.unwrap_bridge());
                        });
                        Ok(())
                    }
                }

                #[allow(non_upper_case_globals)]
                pub static BRIDGE: BRIDGE_EMULATOR = BRIDGE_EMULATOR;

                #jni_code
            }
        }
    };

    TokenStream::from(expanded)
}