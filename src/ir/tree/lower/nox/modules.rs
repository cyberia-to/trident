//! Module identity and conditional compilation for the nox lowering.

use super::*;

impl NoxCompiler {
    /// Lower type-checked modules in dependency order, preserving the lexical
    /// module of every function, constant and struct. `entry` selects the
    /// program (or public library entry) and `flags` select active items.
    pub fn compile_modules(
        &mut self,
        files: &[&ast::File],
        entry: &ast::File,
        flags: &BTreeSet<String>,
    ) -> Result<Noun, String> {
        // A compiler may be reused; symbols and state from its last program
        // must not affect this one.
        *self = Self::new();
        let mut aliases = BTreeMap::new();
        for file in files {
            self.current_module = file.name.node.clone();
            self.module_aliases
                .insert(self.current_module.clone(), aliases.clone());
            for item in &file.items {
                if !active(&item.node, flags) {
                    continue;
                }
                match &item.node {
                    Item::Const(c) => {
                        if let Expr::Literal(Literal::Integer(v)) = &c.value.node {
                            self.constants.insert(self.symbol(&c.name.node), *v);
                        }
                    }
                    Item::Fn(f) => {
                        let mut f = f.clone();
                        f.name.node = self.symbol(&f.name.node);
                        for p in &mut f.params {
                            p.ty.node = self.qualified_type(&p.ty.node);
                        }
                        if let Some(t) = &mut f.return_ty {
                            t.node = self.qualified_type(&t.node);
                        }
                        self.fns.insert(f.name.node.clone(), f);
                    }
                    Item::Struct(s) => {
                        let fields = s
                            .fields
                            .iter()
                            .map(|f| (f.name.node.clone(), self.qualified_type(&f.ty.node)))
                            .collect();
                        self.structs.insert(self.symbol(&s.name.node), fields);
                    }
                    Item::Event(_) => {}
                }
            }
            // This mirrors TypeChecker::import_module: dependent modules see
            // both the full name and the last-segment alias, in dependency
            // order. The function body keeps the aliases of its own module.
            aliases.insert(file.name.node.clone(), file.name.node.clone());
            if let Some(short) = file.name.node.rsplit('.').next() {
                aliases.insert(short.to_string(), file.name.node.clone());
            }
        }
        self.current_module = entry.name.node.clone();
        let candidates = || {
            entry.items.iter().filter_map(|item| {
                if !active(&item.node, flags) {
                    return None;
                }
                match &item.node {
                    Item::Fn(f) => Some(f),
                    _ => None,
                }
            })
        };
        let f = candidates()
            .find(|f| f.name.node == "main")
            .or_else(|| candidates().find(|f| f.is_pub && f.body.is_some()))
            .ok_or_else(|| "no entry function found".to_string())?;
        let f = self
            .fns
            .get(&self.symbol(&f.name.node))
            .cloned()
            .ok_or_else(|| "entry module was not provided to nox lowering".to_string())?;
        self.compile_fn(&f)
    }

    /// Turn a name from the current body into its global identity. Dotted
    /// names are module-qualified; bare names belong to the current module.
    pub(super) fn symbol(&self, name: &str) -> String {
        if let Some((prefix, member)) = name.rsplit_once('.') {
            let module = self
                .module_aliases
                .get(&self.current_module)
                .and_then(|a| a.get(prefix))
                .map(String::as_str)
                .unwrap_or(prefix);
            format!("{module}.{member}")
        } else if self.current_module.is_empty() {
            name.to_string()
        } else {
            format!("{}.{name}", self.current_module)
        }
    }

    pub(super) fn qualified_type(&self, ty: &ast::Type) -> ast::Type {
        match ty {
            ast::Type::Named(path) => ast::Type::Named(ast::ModulePath(
                self.symbol(&path.as_dotted())
                    .split('.')
                    .map(str::to_string)
                    .collect(),
            )),
            ast::Type::Array(inner, size) => {
                ast::Type::Array(Box::new(self.qualified_type(inner)), size.clone())
            }
            ast::Type::Tuple(ts) => {
                ast::Type::Tuple(ts.iter().map(|t| self.qualified_type(t)).collect())
            }
            _ => ty.clone(),
        }
    }
}

fn active(item: &Item, flags: &BTreeSet<String>) -> bool {
    let cfg = match item {
        Item::Fn(f) => &f.cfg,
        Item::Const(c) => &c.cfg,
        Item::Struct(s) => &s.cfg,
        Item::Event(e) => &e.cfg,
    };
    cfg.as_ref().is_none_or(|flag| flags.contains(&flag.node))
}
