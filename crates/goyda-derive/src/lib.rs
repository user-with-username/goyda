extern crate proc_macro;

mod utils;
mod scanner;
mod transformer;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, LitStr};
use syn::visit_mut::VisitMut;
use transformer::{ReactivityGraphTransformer, UiMacroTransformer};

/// Rewrites `let mut x = ...;` into `Signal`/`Memo` state and every
/// `stack!`/`parse_children!` use of a variable that got that treatment
/// into the matching `.get()`/`.set()`/`.update()`/`.call()` - see
/// [`ReactivityGraphTransformer`] and [`UiMacroTransformer`]. Shared by
/// `#[page]` (which also registers a route) and `#[component]` (which
/// doesn't - see its own doc comment for why that's the only difference):
/// a page *is* a component, just one with a path attached, so both need
/// exactly this same rewrite and nothing else.
///
/// `key_namespace` (`Some(route)` for `#[page(route)]`, `None` for
/// `#[component]`) makes every top-level `Signal` this function declares
/// survive being un-mounted and re-mounted (navigating away and back, or a
/// windows hot-reload dylib swap) instead of resetting - see
/// `goyda::reactive::Signal::new_keyed`. `#[component]` doesn't get this:
/// unlike a page (mounted at most once at a time, uniquely identified by
/// its route), a component can be called several times in one render (e.g.
/// once per item in a list), and a key derived only from the function's own
/// name/declaration order can't tell those calls apart - so it stays
/// fresh-state-per-render, same as before.
fn transform_reactive_fn(input_fn: &mut ItemFn, key_namespace: Option<String>) {
    let mut graph_transformer = ReactivityGraphTransformer::new(key_namespace);
    graph_transformer.build_dependency_graph(input_fn);

    let mut_vars_cache = graph_transformer.all_mut_vars.clone();
    graph_transformer.visit_item_fn_mut(input_fn);

    let mut ui_transformer = UiMacroTransformer { all_mut_vars: mut_vars_cache };
    ui_transformer.visit_item_fn_mut(input_fn);
}

/// A reusable, composable piece of UI: an ordinary function - taking
/// whatever arguments it needs (a label, an initial value, a callback,
/// ...) - that builds and returns a [`Component`](::goyda::Component),
/// usually via a `stack! { direction: ..., ... }` of its own (though
/// nothing requires one - returning a single `Component::text(...)`
/// directly is just as valid). `direction` itself is never this macro's
/// concern: it's an ordinary argument to whichever `stack!` call the
/// component's body makes, exactly as it would be in a `#[page]` - a
/// component embedded as one child among a parent's own `stack!` only
/// hands over *that* child's subtree, never influencing the parent's own
/// layout direction or vice versa.
///
/// Unlike [`page`], `#[component]` takes no route and registers nothing -
/// call the function directly, anywhere a `Component` is expected: as a
/// page's whole return value, or (most commonly) as one entry in another
/// `stack!`'s children list, e.g. `my_button_group("Save", on_save)` right
/// alongside `text { ... }`/`button { ... }` - `parse_children!`'s
/// catch-all arm already accepts any `Component`-valued expression there,
/// so no special wiring is needed to nest one.
#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_fn = parse_macro_input!(item as ItemFn);
    transform_reactive_fn(&mut input_fn, None);
    TokenStream::from(quote! { #input_fn })
}

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