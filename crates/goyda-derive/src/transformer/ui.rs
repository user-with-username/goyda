use syn::{visit_mut::VisitMut, Expr, Ident};
use quote::quote;
use std::collections::HashSet;
use proc_macro2::{TokenStream as TokenStream2, TokenTree, Group};

pub struct UiMacroTransformer {
    pub all_mut_vars: HashSet<Ident>,
}

impl UiMacroTransformer {
    pub fn process_macro_tokens(
        &self, 
        tokens: TokenStream2, 
        counter: &mut usize, 
        pre_clones: &mut Vec<TokenStream2>
    ) -> TokenStream2 {
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

                            if matches!(member_name.as_str(), "get" | "set" | "update" | "clone" | "call") {
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

                            if let Some(group) = call_group {
                                let processed_args = self.process_macro_tokens(group.stream(), counter, pre_clones);

                                let call_shadow = syn::Ident::new(
                                    &format!("{}_call_shadow_{}", ident, counter),
                                    ident.span()
                                );
                                *counter += 1;
                                pre_clones.push(quote! { let #call_shadow = #ident.clone(); });

                                let replacement = quote! {
                                    #call_shadow.call(|v| v.#member(#processed_args))
                                };
                                let replacement_trees: Vec<TokenTree> = replacement.into_iter().collect();
                                trees.splice(i..=(i + 3), replacement_trees.clone());
                                i += replacement_trees.len();
                                continue;
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

                                let call_shadow = syn::Ident::new(
                                    &format!("{}_call_shadow_{}", ident, counter),
                                    ident.span()
                                );
                                *counter += 1;
                                pre_clones.push(quote! { let #call_shadow = #ident.clone(); });

                                let replacement = quote! {
                                    #call_shadow.call(|v| v.#member = #right_stream)
                                };
                                let replacement_trees: Vec<TokenTree> = replacement.into_iter().collect();
                                trees.splice(i..j, replacement_trees.clone());
                                i += replacement_trees.len();
                                continue;
                            }

                            let call_shadow = syn::Ident::new(
                                &format!("{}_call_shadow_{}", ident, counter),
                                ident.span()
                            );
                            *counter += 1;
                            pre_clones.push(quote! { let #call_shadow = #ident.clone(); });

                            let replacement = quote! { #call_shadow.call(|v| v.#member) };
                            let replacement_trees: Vec<TokenTree> = replacement.into_iter().collect();
                            trees.splice(i..=(i + 2), replacement_trees.clone());
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
                    let pre_clones_stream: TokenStream2 = pre_clones.into_iter().collect();
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