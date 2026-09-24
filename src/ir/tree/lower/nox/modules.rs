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
        self.compile_modules_profile(files, entry, flags, false, &BTreeMap::new())
    }

    /// Compile `fn main(input: Noun) -> Noun` for ART1 raw profiles 0/0.
    pub fn compile_raw_modules(
        &mut self,
        files: &[&ast::File],
        entry: &ast::File,
        flags: &BTreeSet<String>,
    ) -> Result<Noun, String> {
        self.compile_modules_profile(files, entry, flags, true, &BTreeMap::new())
    }

    pub(crate) fn compile_raw_modules_with_origins(
        &mut self,
        files: &[&ast::File],
        entry: &ast::File,
        flags: &BTreeSet<String>,
        origins: &BTreeMap<String, (String, Vec<u64>)>,
    ) -> Result<Noun, String> {
        self.compile_modules_profile(files, entry, flags, true, origins)
    }

    fn compile_modules_profile(
        &mut self,
        files: &[&ast::File],
        entry: &ast::File,
        flags: &BTreeSet<String>,
        raw: bool,
        origins: &BTreeMap<String, (String, Vec<u64>)>,
    ) -> Result<Noun, String> {
        // A compiler may be reused; symbols and state from its last program
        // must not affect this one.
        *self = Self::new();
        self.raw_entry = raw;
        if raw && !entry.declarations.is_empty() {
            return Err("raw ART1 entry has no flat I/O declarations".into());
        }
        let mut aliases = BTreeMap::new();
        let mut functions = BTreeMap::new();
        let mut constants = BTreeMap::new();
        for file in files {
            self.current_module = file.name.node.clone();
            self.function_aliases
                .insert(self.current_module.clone(), functions.clone());
            self.constant_aliases
                .insert(self.current_module.clone(), constants.clone());
            self.module_aliases
                .insert(self.current_module.clone(), aliases.clone());
            // Resolve lexical size constants independently of declaration order.
            for item in &file.items {
                if active(&item.node, flags) {
                    if let Item::Const(c) = &item.node {
                        if let Expr::Literal(Literal::Integer(v)) = &c.value.node {
                            self.constants.insert(self.symbol(&c.name.node), *v);
                        }
                    }
                }
            }
            for item in &file.items {
                if !active(&item.node, flags) {
                    continue;
                }
                match &item.node {
                    Item::Const(_) => {}
                    Item::Fn(f) => {
                        let mut f = f.clone();
                        f.name.node = self.symbol(&f.name.node);
                        let generics = f.type_params.iter().map(|p| p.node.clone()).collect();
                        for p in &mut f.params {
                            p.ty.node = self.qualified_type_preserving(&p.ty.node, &generics);
                        }
                        if let Some(t) = &mut f.return_ty {
                            t.node = self.qualified_type_preserving(&t.node, &generics);
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
            for item in &file.items {
                if !active(&item.node, flags) {
                    continue;
                }
                if let Item::Fn(f) = &item.node {
                    if f.is_pub {
                        let full = &file.name.node;
                        let short = full.rsplit('.').next().unwrap_or(full);
                        let canonical = format!("{full}.{}", f.name.node);
                        functions.insert(canonical.clone(), canonical.clone());
                        if short != full {
                            functions.insert(format!("{short}.{}", f.name.node), canonical);
                        }
                    }
                }
            }
            for item in &file.items {
                if !active(&item.node, flags) {
                    continue;
                }
                if let Item::Const(c) = &item.node {
                    if c.is_pub {
                        let full = &file.name.node;
                        let short = full.rsplit('.').next().unwrap_or(full);
                        let canonical = format!("{full}.{}", c.name.node);
                        constants.insert(canonical.clone(), canonical.clone());
                        if short != full {
                            constants.insert(format!("{short}.{}", c.name.node), canonical);
                        }
                    }
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
        self.scan_state_functions();
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
        if raw {
            native::compile(self, &f, origins)
        } else {
            self.compile_fn(&f)
        }
    }

    pub(super) fn function_symbol(&self, name: &str) -> String {
        self.function_aliases
            .get(&self.current_module)
            .and_then(|aliases| aliases.get(name))
            .cloned()
            .unwrap_or_else(|| self.symbol(name))
    }

    pub(super) fn constant_symbol(&self, name: &str) -> String {
        self.constant_aliases
            .get(&self.current_module)
            .and_then(|aliases| aliases.get(name))
            .cloned()
            .unwrap_or_else(|| self.symbol(name))
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
        self.qualified_type_preserving(ty, &BTreeSet::new())
    }

    fn qualified_type_preserving(&self, ty: &ast::Type, generics: &BTreeSet<String>) -> ast::Type {
        match ty {
            ast::Type::Named(path) => ast::Type::Named(ast::ModulePath(
                self.symbol(&path.as_dotted())
                    .split('.')
                    .map(str::to_string)
                    .collect(),
            )),
            ast::Type::Array(inner, size) => ast::Type::Array(
                Box::new(self.qualified_type_preserving(inner, generics)),
                self.qualified_size(size, generics),
            ),
            ast::Type::Tuple(ts) => ast::Type::Tuple(
                ts.iter()
                    .map(|t| self.qualified_type_preserving(t, generics))
                    .collect(),
            ),
            _ => ty.clone(),
        }
    }

    fn qualified_size(&self, size: &ast::ArraySize, generics: &BTreeSet<String>) -> ast::ArraySize {
        use ast::ArraySize;
        match size {
            ArraySize::Param(name) if !generics.contains(name) => self
                .constants
                .get(&self.constant_symbol(name))
                .map(|n| ArraySize::Literal(*n))
                // Keep lexical ownership even if a constant is unresolved;
                // never capture an equally named entry-module constant.
                .unwrap_or_else(|| ArraySize::Param(self.constant_symbol(name))),
            ArraySize::Add(a, b) | ArraySize::Mul(a, b) => {
                let a = self.qualified_size(a, generics);
                let b = self.qualified_size(b, generics);
                if let (ArraySize::Literal(x), ArraySize::Literal(y)) = (&a, &b) {
                    let value = if matches!(size, ArraySize::Add(..)) {
                        x.checked_add(*y)
                    } else {
                        x.checked_mul(*y)
                    };
                    if let Some(value) = value {
                        return ArraySize::Literal(value);
                    }
                }
                if matches!(size, ArraySize::Add(..)) {
                    ArraySize::Add(Box::new(a), Box::new(b))
                } else {
                    ArraySize::Mul(Box::new(a), Box::new(b))
                }
            }
            _ => size.clone(),
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

#[cfg(test)]
mod size_tests {
    use super::*;

    #[test]
    fn lexical_sizes_preserve_generics_and_never_wrap_arithmetic() {
        use ast::ArraySize as S;
        let mut compiler = NoxCompiler::new();
        compiler.current_module = "library".into();
        compiler.constants.insert("library.N".into(), 2);
        let n = S::Param("N".into());
        assert!(matches!(
            compiler.qualified_size(&n, &BTreeSet::new()),
            S::Literal(2)
        ));
        assert!(
            matches!(compiler.qualified_size(&n, &BTreeSet::from(["N".into()])), S::Param(x) if x == "N")
        );
        for size in [
            S::Add(Box::new(S::Literal(u64::MAX)), Box::new(S::Literal(1))),
            S::Mul(Box::new(S::Literal(u64::MAX)), Box::new(S::Literal(2))),
        ] {
            assert!(!matches!(
                compiler.qualified_size(&size, &BTreeSet::new()),
                S::Literal(_)
            ));
        }
        assert!(
            matches!(compiler.qualified_size(&S::Param("unknown".into()), &BTreeSet::new()), S::Param(x) if x == "library.unknown")
        );
    }
}
