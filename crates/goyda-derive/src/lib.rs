extern crate proc_macro;

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