extern crate proc_macro;

mod android;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, visit_mut::VisitMut, visit::Visit, Expr, 
    ItemFn, LitStr, Local, Ident, Stmt
};
use std::collections::{HashSet, HashMap};

use petgraph::graph::DiGraph;

fn as_mut_pat_ident(pat: &syn::Pat) -> Option<&syn::PatIdent> {
    match pat {
        syn::Pat::Ident(pat_ident) if pat_ident.mutability.is_some() => Some(pat_ident),
        syn::Pat::Type(pat_type) => as_mut_pat_ident(&pat_type.pat),
        _ => None,
    }
}

struct DependencyScanner {
    found_idents: HashSet<Ident>,
    filter_list: HashSet<Ident>,
}

impl<'ast> Visit<'ast> for DependencyScanner {
    fn visit_ident(&mut self, id: &'ast Ident) {
        if self.filter_list.contains(id) {
            self.found_idents.insert(id.clone());
        }
    }
}

struct ReactivityGraphTransformer {
    all_mut_vars: HashSet<Ident>,
    graph: DiGraph<Ident, ()>,
    node_indices: HashMap<Ident, petgraph::graph::NodeIndex>,
}

impl ReactivityGraphTransformer {
    fn new() -> Self {
        Self {
            all_mut_vars: HashSet::new(),
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
        }
    }

    fn build_dependency_graph(&mut self, item_fn: &ItemFn) {
        struct MutCollector { vars: HashSet<Ident> }
        impl<'ast> Visit<'ast> for MutCollector {
            fn visit_local(&mut self, local: &'ast Local) {
                if let Some(pat_ident) = as_mut_pat_ident(&local.pat) {
                    self.vars.insert(pat_ident.ident.clone());
                }
                syn::visit::visit_local(self, local);
            }
        }

        let mut collector = MutCollector { vars: HashSet::new() };
        collector.visit_item_fn(item_fn);
        self.all_mut_vars = collector.vars.clone();

        for var in &self.all_mut_vars {
            let idx = self.graph.add_node(var.clone());
            self.node_indices.insert(var.clone(), idx);
        }

        struct EdgeCollector<'a> {
            all_mut_vars: &'a HashSet<Ident>,
            current_local_target: Option<Ident>,
            edges: Vec<(Ident, Ident)>,
        }

        impl<'ast, 'a> Visit<'ast> for EdgeCollector<'a> {
            fn visit_local(&mut self, local: &'ast Local) {
                let old_target = self.current_local_target.clone();
                if let Some(pat_ident) = as_mut_pat_ident(&local.pat) {
                    if self.all_mut_vars.contains(&pat_ident.ident) {
                        self.current_local_target = Some(pat_ident.ident.clone());
                    }
                }
                if let Some(init) = &local.init {
                    syn::visit::visit_expr(self, &init.expr);
                }
                self.current_local_target = old_target;
            }

            fn visit_ident(&mut self, id: &'ast Ident) {
                if let Some(target) = &self.current_local_target {
                    if self.all_mut_vars.contains(id) && id != target {
                        self.edges.push((id.clone(), target.clone()));
                    }
                }
            }
        }

        let mut edge_collector = EdgeCollector {
            all_mut_vars: &self.all_mut_vars,
            current_local_target: None,
            edges: Vec::new(),
        };
        edge_collector.visit_item_fn(item_fn);

        for (from, to) in edge_collector.edges {
            if let (Some(&f_idx), Some(&t_idx)) = (self.node_indices.get(&from), self.node_indices.get(&to)) {
                self.graph.add_edge(f_idx, t_idx, ());
            }
        }
    }
}

impl VisitMut for ReactivityGraphTransformer {
    fn visit_stmt_mut(&mut self, stmt: &mut Stmt) {
        if let Stmt::Local(local) = stmt {
            let annotated_ty: Option<syn::Type> = match &local.pat {
                syn::Pat::Type(pat_type) => Some((*pat_type.ty).clone()),
                _ => None,
            };

            if let Some(pat_ident) = as_mut_pat_ident(&local.pat) {
                if self.all_mut_vars.contains(&pat_ident.ident) {
                    let ident = pat_ident.ident.clone();
                    let init_expr = match &local.init {
                        Some(local_init) => local_init.expr.clone(),
                        None => panic!("Reactive variables must be initialized immediately"),
                    };
                    let init_expr = &init_expr;

                    let incoming_edges: Vec<_> = self.node_indices.get(&ident)
                        .map(|&idx| self.graph.neighbors_directed(idx, petgraph::Direction::Incoming).collect())
                        .unwrap_or_default();

                    if incoming_edges.is_empty() {
                        *stmt = if let Some(ty) = &annotated_ty {
                            syn::parse2(quote! {
                                let #ident: ::goyda::reactive::Signal<#ty> = ::goyda::reactive::Signal::new(#init_expr);
                            }).unwrap()
                        } else {
                            syn::parse2(quote! {
                                let #ident = ::goyda::reactive::Signal::new(#init_expr);
                            }).unwrap()
                        };
                    } else {
                        let mut scanner = DependencyScanner {
                            found_idents: HashSet::new(),
                            filter_list: self.all_mut_vars.clone(),
                        };
                        scanner.visit_expr(init_expr);

                        let mut clones = Vec::new();
                        let mut shadow_mappings = HashMap::new();
                        let mut counter = 0;

                        for dep in scanner.found_idents {
                            let shadow = syn::Ident::new(&format!("{}_memo_shadow_{}", dep, counter), dep.span());
                            counter += 1;
                            clones.push(quote! { let #shadow = #dep.clone(); });
                            shadow_mappings.insert(dep, shadow);
                        }

                        let mut body_expr = init_expr.clone();
                        struct MemoReplacer { mappings: HashMap<Ident, Ident> }
                        impl VisitMut for MemoReplacer {
                            fn visit_expr_mut(&mut self, expr: &mut Expr) {
                                if let Expr::Path(ep) = expr {
                                    if let Some(id) = ep.path.get_ident() {
                                        if let Some(shadow_id) = self.mappings.get(id) {
                                            *expr = syn::parse2(quote! { #shadow_id.get() }).unwrap();
                                            return;
                                        }
                                    }
                                }
                                syn::visit_mut::visit_expr_mut(self, expr);
                            }
                        }
                        let mut replacer = MemoReplacer { mappings: shadow_mappings };
                        replacer.visit_expr_mut(&mut body_expr);

                        let clones_stream: proc_macro2::TokenStream = clones.into_iter().collect();

                        *stmt = if let Some(ty) = &annotated_ty {
                            syn::parse2(quote! {
                                let #ident: ::goyda::reactive::Memo<#ty> = {
                                    #clones_stream
                                    ::goyda::reactive::Memo::new(move || #body_expr)
                                };
                            }).unwrap()
                        } else {
                            syn::parse2(quote! {
                                let #ident = {
                                    #clones_stream
                                    ::goyda::reactive::Memo::new(move || #body_expr)
                                };
                            }).unwrap()
                        };
                    }
                    return;
                }
            }
        }
        syn::visit_mut::visit_stmt_mut(self, stmt);
    }
}

struct UiMacroTransformer {
    all_mut_vars: HashSet<Ident>,
}

impl UiMacroTransformer {
    fn process_macro_tokens(
        &self, 
        tokens: proc_macro2::TokenStream, 
        counter: &mut usize, 
        pre_clones: &mut Vec<proc_macro2::TokenStream>
    ) -> proc_macro2::TokenStream {
        use proc_macro2::{TokenStream as TokenStream2, TokenTree, Group};
        
        let mut trees: Vec<TokenTree> = tokens.into_iter().collect();
        let mut i = 0;

        while i < trees.len() {
            if let TokenTree::Group(group) = &trees[i] {
                let transformed_inner = self.process_macro_tokens(group.stream(), counter, pre_clones);
                let mut new_group = Group::new(group.delimiter(), transformed_inner);
                new_group.set_span(group.span());
                trees[i] = TokenTree::Group(new_group);
                i += 1;
                continue;
            }

            if let TokenTree::Ident(ident) = &trees[i] {
                if self.all_mut_vars.contains(ident) {
                    let is_property_key = if i + 1 < trees.len() {
                        if let TokenTree::Punct(p) = &trees[i + 1] {
                            p.as_char() == ':'
                        } else { false }
                    } else { false };

                    let next_is_dot = matches!(&trees.get(i + 1), Some(TokenTree::Punct(p)) if p.as_char() == '.');

                    if !is_property_key && next_is_dot {
                        if let Some(TokenTree::Ident(member)) = trees.get(i + 2).cloned() {
                            let member_name = member.to_string();

                            if matches!(member_name.as_str(), "get" | "set" | "update" | "clone") {
                                let shadow_ident = syn::Ident::new(
                                    &format!("{}_ui_shadow_{}", ident, counter),
                                    ident.span()
                                );
                                *counter += 1;
                                pre_clones.push(quote! { let #shadow_ident = #ident.clone(); });
                                trees[i] = TokenTree::Ident(shadow_ident);
                                i += 1;
                                continue;
                            }

                            let call_group = match trees.get(i + 3) {
                                Some(TokenTree::Group(g)) if g.delimiter() == proc_macro2::Delimiter::Parenthesis => Some(g.clone()),
                                _ => None,
                            };

                            const MUTATING_METHODS: &[&str] = &[
                                // Vec / VecDeque / slice
                                "push", "pop", "insert", "remove", "swap_remove", "clear",
                                "extend", "extend_from_slice", "append", "truncate",
                                "retain", "retain_mut", "drain", "dedup", "dedup_by",
                                "dedup_by_key", "sort", "sort_by", "sort_by_key",
                                "sort_unstable", "sort_unstable_by", "sort_unstable_by_key",
                                "reverse", "resize", "resize_with", "fill", "fill_with",
                                "rotate_left", "rotate_right", "swap", "split_off",
                                "push_front", "push_back", "pop_front", "pop_back",
                                // String
                                "push_str", "insert_str", "replace_range", "splice",
                                // HashMap / HashSet / BTreeMap / BTreeSet
                                "toggle",
                            ];

                            if let Some(group) = call_group {
                                let is_mutating = MUTATING_METHODS.contains(&member_name.as_str());
                                let processed_args = self.process_macro_tokens(group.stream(), counter, pre_clones);

                                if is_mutating {
                                    let mutation_shadow = syn::Ident::new(
                                        &format!("{}_mut_shadow_{}", ident, counter),
                                        ident.span()
                                    );
                                    *counter += 1;
                                    pre_clones.push(quote! { let #mutation_shadow = #ident.clone(); });

                                    let replacement = quote! {
                                        #mutation_shadow.update(|v| { v.#member(#processed_args); })
                                    };
                                    let replacement_trees: Vec<TokenTree> = replacement.into_iter().collect();
                                    trees.splice(i..=(i + 3), replacement_trees.clone());
                                    i += replacement_trees.len();
                                    continue;
                                } else {
                                    let shadow_ident = syn::Ident::new(
                                        &format!("{}_ui_shadow_{}", ident, counter),
                                        ident.span()
                                    );
                                    *counter += 1;
                                    pre_clones.push(quote! { let #shadow_ident = #ident.clone(); });

                                    let new_call_group = Group::new(proc_macro2::Delimiter::Parenthesis, processed_args);
                                    trees[i + 3] = TokenTree::Group(new_call_group);

                                    let replacement = quote! { #shadow_ident.get() };
                                    let replacement_trees: Vec<TokenTree> = replacement.into_iter().collect();
                                    trees.splice(i..=i, replacement_trees.clone());
                                    i += replacement_trees.len();
                                    continue;
                                }
                            }

                            let is_field_assign = matches!(trees.get(i + 3), Some(TokenTree::Punct(p)) if p.as_char() == '=')
                                && !matches!(trees.get(i + 4), Some(TokenTree::Punct(p)) if p.as_char() == '=');

                            if is_field_assign {
                                let mut right_tokens = Vec::new();
                                let mut j = i + 4;
                                while j < trees.len() {
                                    if let TokenTree::Punct(p) = &trees[j] {
                                        if p.as_char() == ',' || p.as_char() == ';' { break; }
                                    }
                                    right_tokens.push(trees[j].clone());
                                    j += 1;
                                }
                                let right_stream: TokenStream2 = self.process_macro_tokens(
                                    right_tokens.into_iter().collect(),
                                    counter,
                                    pre_clones,
                                );

                                let mutation_shadow = syn::Ident::new(
                                    &format!("{}_mut_shadow_{}", ident, counter),
                                    ident.span()
                                );
                                *counter += 1;
                                pre_clones.push(quote! { let #mutation_shadow = #ident.clone(); });

                                let replacement = quote! {
                                    #mutation_shadow.update(|v| v.#member = #right_stream)
                                };
                                let replacement_trees: Vec<TokenTree> = replacement.into_iter().collect();
                                trees.splice(i..j, replacement_trees.clone());
                                i += replacement_trees.len();
                                continue;
                            }

                            let shadow_ident = syn::Ident::new(
                                &format!("{}_ui_shadow_{}", ident, counter),
                                ident.span()
                            );
                            *counter += 1;
                            pre_clones.push(quote! { let #shadow_ident = #ident.clone(); });

                            let replacement = quote! { #shadow_ident.get() };
                            let replacement_trees: Vec<TokenTree> = replacement.into_iter().collect();
                            trees.splice(i..=i, replacement_trees.clone());
                            i += replacement_trees.len();
                            continue;
                        }
                    }

                    let is_plain_eq = i + 1 < trees.len()
                        && matches!(&trees[i + 1], TokenTree::Punct(p) if p.as_char() == '=')
                        && !matches!(trees.get(i + 2), Some(TokenTree::Punct(p)) if p.as_char() == '=');

                    if !is_property_key {
                        if i + 2 < trees.len() {
                            if let (TokenTree::Punct(p1), TokenTree::Punct(p2)) = (&trees[i + 1], &trees[i + 2]) {
                                let op = p1.as_char();
                                if p2.as_char() == '=' && (op == '+' || op == '-' || op == '*' || op == '/') {
                                    let mut right_tokens = Vec::new();
                                    let mut j = i + 3;
                                    while j < trees.len() {
                                        if let TokenTree::Punct(p) = &trees[j] {
                                            if p.as_char() == ',' || p.as_char() == ';' { break; }
                                        }
                                        right_tokens.push(trees[j].clone());
                                        j += 1;
                                    }
                                    let right_stream: TokenStream2 = self.process_macro_tokens(
                                        right_tokens.into_iter().collect(),
                                        counter,
                                        pre_clones,
                                    );
                                    let op_punct = proc_macro2::Punct::new(op, proc_macro2::Spacing::Joint);
                                    
                                    let mutation_shadow = syn::Ident::new(
                                        &format!("{}_mut_shadow_{}", ident, counter),
                                        ident.span()
                                    );
                                    *counter += 1;

                                    pre_clones.push(quote! {
                                        let #mutation_shadow = #ident.clone();
                                    });

                                    let replacement = quote! {
                                        #mutation_shadow.update(|v| *v #op_punct= #right_stream)
                                    };
                                    let replacement_trees: Vec<TokenTree> = replacement.into_iter().collect();
                                    trees.splice(i..j, replacement_trees.clone());
                                    i += replacement_trees.len();
                                    continue;
                                }
                            }
                        }

                        if is_plain_eq {
                            let mut right_tokens = Vec::new();
                            let mut j = i + 2;
                            while j < trees.len() {
                                if let TokenTree::Punct(p) = &trees[j] {
                                    if p.as_char() == ',' || p.as_char() == ';' { break; }
                                }
                                right_tokens.push(trees[j].clone());
                                j += 1;
                            }
                            let right_stream: TokenStream2 = self.process_macro_tokens(
                                right_tokens.into_iter().collect(),
                                counter,
                                pre_clones,
                            );

                            let mutation_shadow = syn::Ident::new(
                                &format!("{}_mut_shadow_{}", ident, counter),
                                ident.span()
                            );
                            *counter += 1;

                            pre_clones.push(quote! {
                                let #mutation_shadow = #ident.clone();
                            });

                            let replacement = quote! {
                                #mutation_shadow.set(#right_stream)
                            };
                            let replacement_trees: Vec<TokenTree> = replacement.into_iter().collect();
                            trees.splice(i..j, replacement_trees.clone());
                            i += replacement_trees.len();
                            continue;
                        }

                        let shadow_ident = syn::Ident::new(
                            &format!("{}_ui_shadow_{}", ident, counter),
                            ident.span()
                        );
                        *counter += 1;

                        pre_clones.push(quote! {
                            let #shadow_ident = #ident.clone();
                        });

                        let replacement = quote! { #shadow_ident.get() };
                        let replacement_trees: Vec<TokenTree> = replacement.into_iter().collect();
                        trees.splice(i..=i, replacement_trees.clone());
                        i += replacement_trees.len();
                        continue;
                    }
                }
            }
            i += 1;
        }

        trees.into_iter().collect()
    }
}

impl VisitMut for UiMacroTransformer {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Macro(expr_macro) = expr {
            let macro_name = expr_macro.mac.path.segments.last().unwrap().ident.to_string();
            if macro_name == "stack" || macro_name == "parse_children" {
                let mut counter = 0;
                let mut pre_clones = Vec::new();
                
                let transformed_tokens = self.process_macro_tokens(
                    expr_macro.mac.tokens.clone(), 
                    &mut counter, 
                    &mut pre_clones
                );
                
                expr_macro.mac.tokens = transformed_tokens;

                if !pre_clones.is_empty() {
                    let pre_clones_stream: proc_macro2::TokenStream = pre_clones.into_iter().collect();
                    let current_macro = expr.clone();
                    *expr = syn::parse2(quote! {{
                        #pre_clones_stream
                        #current_macro
                    }}).unwrap();
                }
                return;
            }
        }
        syn::visit_mut::visit_expr_mut(self, expr);
    }
}

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