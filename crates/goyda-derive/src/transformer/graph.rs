use petgraph::graph::DiGraph;
use quote::quote;
use std::collections::{HashMap, HashSet};
use syn::{Expr, Ident, ItemFn, Local, Stmt, visit::Visit, visit_mut::VisitMut};

use crate::scanner::DependencyScanner;
use crate::utils::as_mut_pat_ident;

pub struct ReactivityGraphTransformer {
    pub all_mut_vars: HashSet<Ident>,
    pub graph: DiGraph<Ident, ()>,
    pub node_indices: HashMap<Ident, petgraph::graph::NodeIndex>,
    /// A stable identity for this function's root `Signal`s to persist
    /// their value under across an unmount/remount (see
    /// `goyda::reactive::Signal::new_keyed`) - `Some(route)` for
    /// `#[page(route)]`, `None` for `#[component]`. `None` means every
    /// `Signal` here is the plain, unkeyed kind, reset fresh on every call
    /// (see [`transform_reactive_fn`](crate::transform_reactive_fn)'s doc
    /// comment for why `#[component]` doesn't get this).
    pub key_namespace: Option<String>,
    next_key_index: usize,
}

impl ReactivityGraphTransformer {
    pub fn new(key_namespace: Option<String>) -> Self {
        Self {
            all_mut_vars: HashSet::new(),
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
            key_namespace,
            next_key_index: 0,
        }
    }

    pub fn build_dependency_graph(&mut self, item_fn: &ItemFn) {
        struct MutCollector {
            vars: HashSet<Ident>,
        }
        impl<'ast> Visit<'ast> for MutCollector {
            fn visit_local(&mut self, local: &'ast Local) {
                if let Some(pat_ident) = as_mut_pat_ident(&local.pat) {
                    self.vars.insert(pat_ident.ident.clone());
                }
                syn::visit::visit_local(self, local);
            }
        }

        let mut collector = MutCollector {
            vars: HashSet::new(),
        };
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
                if let Some(pat_ident) = as_mut_pat_ident(&local.pat)
                    && self.all_mut_vars.contains(&pat_ident.ident) {
                        self.current_local_target = Some(pat_ident.ident.clone());
                    }
                if let Some(init) = &local.init {
                    syn::visit::visit_expr(self, &init.expr);
                }
                self.current_local_target = old_target;
            }

            fn visit_ident(&mut self, id: &'ast Ident) {
                if let Some(target) = &self.current_local_target
                    && self.all_mut_vars.contains(id) && id != target {
                        self.edges.push((id.clone(), target.clone()));
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
            if let (Some(&f_idx), Some(&t_idx)) =
                (self.node_indices.get(&from), self.node_indices.get(&to))
            {
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

            if let Some(pat_ident) = as_mut_pat_ident(&local.pat)
                && self.all_mut_vars.contains(&pat_ident.ident) {
                    let ident = pat_ident.ident.clone();
                    let init_expr = match &local.init {
                        Some(local_init) => local_init.expr.clone(),
                        None => panic!("Reactive variables must be initialized immediately"),
                    };
                    let init_expr = &init_expr;

                    let incoming_edges: Vec<_> = self
                        .node_indices
                        .get(&ident)
                        .map(|&idx| {
                            self.graph
                                .neighbors_directed(idx, petgraph::Direction::Incoming)
                                .collect()
                        })
                        .unwrap_or_default();

                    if incoming_edges.is_empty() {
                        let key_literal = self.key_namespace.as_ref().map(|ns| {
                            let key = format!("{ns}#{}", self.next_key_index);
                            self.next_key_index += 1;
                            key
                        });

                        *stmt = if let Some(key) = &key_literal {
                            if let Some(ty) = &annotated_ty {
                                syn::parse2(quote! {
                                    let #ident: ::goyda::reactive::Signal<#ty> = ::goyda::reactive::Signal::new_keyed(#key, #init_expr);
                                }).unwrap()
                            } else {
                                syn::parse2(quote! {
                                    let #ident = ::goyda::reactive::Signal::new_keyed(#key, #init_expr);
                                }).unwrap()
                            }
                        } else if let Some(ty) = &annotated_ty {
                            syn::parse2(quote! {
                                let #ident: ::goyda::reactive::Signal<#ty> = ::goyda::reactive::Signal::new(#init_expr);
                            }).unwrap()
                        } else {
                            syn::parse2(quote! {
                                let #ident = ::goyda::reactive::Signal::new(#init_expr);
                            })
                            .unwrap()
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
                            let shadow = syn::Ident::new(
                                &format!("{}_memo_shadow_{}", dep, counter),
                                dep.span(),
                            );
                            counter += 1;
                            clones.push(quote! { let #shadow = #dep.clone(); });
                            shadow_mappings.insert(dep, shadow);
                        }

                        let mut body_expr = init_expr.clone();
                        struct MemoReplacer {
                            mappings: HashMap<Ident, Ident>,
                        }
                        impl VisitMut for MemoReplacer {
                            fn visit_expr_mut(&mut self, expr: &mut Expr) {
                                if let Expr::Path(ep) = expr
                                    && let Some(id) = ep.path.get_ident()
                                        && let Some(shadow_id) = self.mappings.get(id) {
                                            *expr =
                                                syn::parse2(quote! { #shadow_id.get() }).unwrap();
                                            return;
                                        }
                                syn::visit_mut::visit_expr_mut(self, expr);
                            }
                        }
                        let mut replacer = MemoReplacer {
                            mappings: shadow_mappings,
                        };
                        replacer.visit_expr_mut(&mut body_expr);

                        let clones_stream: proc_macro2::TokenStream = clones.into_iter().collect();

                        *stmt = if let Some(ty) = &annotated_ty {
                            syn::parse2(quote! {
                                let #ident: ::goyda::reactive::Memo<#ty> = {
                                    #clones_stream
                                    ::goyda::reactive::Memo::new(move || #body_expr)
                                };
                            })
                            .unwrap()
                        } else {
                            syn::parse2(quote! {
                                let #ident = {
                                    #clones_stream
                                    ::goyda::reactive::Memo::new(move || #body_expr)
                                };
                            })
                            .unwrap()
                        };
                    }
                    return;
                }
        }
        syn::visit_mut::visit_stmt_mut(self, stmt);
    }
}
