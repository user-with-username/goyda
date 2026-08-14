use std::collections::HashSet;
use syn::{Ident, visit::Visit};

pub struct DependencyScanner {
    pub found_idents: HashSet<Ident>,
    pub filter_list: HashSet<Ident>,
}

impl<'ast> Visit<'ast> for DependencyScanner {
    fn visit_ident(&mut self, id: &'ast Ident) {
        if self.filter_list.contains(id) {
            self.found_idents.insert(id.clone());
        }
    }
}
