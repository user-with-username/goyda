//! Attribute macros for declaring Goyda components and pages.

extern crate proc_macro;

mod utils;
mod scanner;
mod transformer;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, LitStr};
use syn::visit_mut::VisitMut;
use transformer::{ReactivityGraphTransformer, UiMacroTransformer};

fn transform_reactive_fn(input_fn: &mut ItemFn, key_namespace: Option<String>) {
    let mut graph_transformer = ReactivityGraphTransformer::new(key_namespace);
    graph_transformer.build_dependency_graph(input_fn);

    let mut_vars_cache = graph_transformer.all_mut_vars.clone();
    graph_transformer.visit_item_fn_mut(input_fn);

    let mut ui_transformer = UiMacroTransformer { all_mut_vars: mut_vars_cache };
    ui_transformer.visit_item_fn_mut(input_fn);
}

/// Marks a function as a reusable UI component.
///
/// The function may take any arguments (labels, initial values, callbacks,
/// ...) and must return a `Component`. `let mut`
/// locals inside the body become reactive state automatically, so plain
/// assignments update the UI. Call the function directly wherever a
/// `Component` is expected, including as a child inside another `stack!`.
///
/// ```rust
/// #[component]
/// fn counter(label: &str) -> Component {
///     let mut count = 0;
///     stack! {
///         text { "{label}: {count}" }
///         button { on_click: move || count += 1, "Increment" }
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_fn = parse_macro_input!(item as ItemFn);
    transform_reactive_fn(&mut input_fn, None);
    TokenStream::from(quote! { #input_fn })
}

/// Marks a function as an application page, reachable at the given route.
///
/// Like [`component`], the function's `let mut` locals become reactive
/// state, and it must return a `Component`. The page
/// is registered automatically and can be navigated to by its route string;
/// its state is preserved across navigation away and back.
///
/// ```rust
/// #[page("/")]
/// fn home() -> Component {
///     stack! {
///         text { "Welcome" }
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn page(attr: TokenStream, item: TokenStream) -> TokenStream {
    let route_path = parse_macro_input!(attr as LitStr);
    let mut input_fn = parse_macro_input!(item as ItemFn);

    transform_reactive_fn(&mut input_fn, Some(route_path.value()));

    let fn_name = &input_fn.sig.ident;
    let module_name = syn::Ident::new(&format!("__goyda_page_container_{}", fn_name), fn_name.span());
    let register_name = syn::Ident::new(&format!("__GOYDA_PAGE_{}", fn_name), fn_name.span());

    // Registration only - every platform resolves `#[page(...)]` routes the
    // same way, through `goyda::find_page` against the `Page` inventory
    // built up here. Android's native entry point (`JNI_OnLoad`/
    // `nativeInit`) lives once in the `goyda` crate itself instead of being
    // generated per `#[page]`, since a consumer app can register more than
    // one page and two macro-generated `JNI_OnLoad`s in the same crate would
    // collide at link time.
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
        }
    };

    TokenStream::from(expanded)
}