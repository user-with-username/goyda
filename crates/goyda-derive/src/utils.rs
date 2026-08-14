use syn::Pat;

pub fn as_mut_pat_ident(pat: &Pat) -> Option<&syn::PatIdent> {
    match pat {
        Pat::Ident(pat_ident) if pat_ident.mutability.is_some() => Some(pat_ident),
        Pat::Type(pat_type) => as_mut_pat_ident(&pat_type.pat),
        _ => None,
    }
}
