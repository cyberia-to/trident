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
        let scopes =
            crate::resolve::scope::scopes(files).map_err(|errors| errors[0].message.clone())?;
        let resolved = crate::typecheck::constants::resolve_modules(files, flags, &BTreeMap::new())
            .map_err(|errors| {
                errors
                    .iter()
                    .map(|e| e.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            })?;
        let builtins = crate::typecheck::TypeChecker::builtin_return_types(
            &crate::target::TerrainConfig::nox(),
        );
        crate::typecheck::nominal_bindings::validate_modules(files, &scopes, &resolved, flags)
            .map_err(|errors| errors[0].message.clone())?;
        for ((file, constants), scope) in files.iter().zip(&resolved).zip(&scopes) {
            scope
                .validate_names(file, files, flags, &builtins, constants)
                .map_err(|errors| errors[0].message.clone())?;
            self.current_module = file.name.node.clone();
            self.function_aliases.insert(
                self.current_module.clone(),
                scope.function_aliases(files, flags),
            );
            self.constant_aliases.insert(
                self.current_module.clone(),
                constants
                    .visible
                    .iter()
                    .map(|(name, binding)| (name.clone(), binding.canonical_name()))
                    .collect(),
            );
            for binding in constants.locals.values() {
                self.constants.insert(binding.canonical_name(), binding.raw);
                self.constant_types
                    .insert(binding.canonical_name(), binding.ty.clone());
            }
            self.type_aliases.insert(
                self.current_module.clone(),
                scope.type_aliases(file, files, flags),
            );
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
        }
        self.scan_state_functions();
        self.current_module = entry.name.node.clone();
        let candidates = entry.final_functions(flags);
        let f = candidates
            .iter()
            .find(|f| f.name.node == "main")
            .or_else(|| candidates.iter().find(|f| f.is_pub && f.body.is_some()))
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

    pub(super) fn function_symbol(&self, name: &str) -> Option<String> {
        self.function_aliases
            .get(&self.current_module)
            .and_then(|aliases| aliases.get(name))
            .cloned()
            .or_else(|| (!name.contains('.')).then(|| self.symbol(name)))
    }

    pub(super) fn constant_symbol(&self, name: &str) -> Option<String> {
        self.constant_aliases
            .get(&self.current_module)
            .and_then(|aliases| aliases.get(name))
            .cloned()
            .or_else(|| (!name.contains('.')).then(|| self.symbol(name)))
    }

    pub(super) fn function(&self, name: &str) -> Option<&FnDef> {
        self.function_symbol(name)
            .and_then(|symbol| self.fns.get(&symbol))
    }

    pub(super) fn constant_value(&self, name: &str) -> Option<u64> {
        self.constant_symbol(name)
            .and_then(|symbol| self.constants.get(&symbol))
            .copied()
    }

    /// Qualify a local source name. Imported names require an explicit alias.
    pub(super) fn symbol(&self, name: &str) -> String {
        if self.current_module.is_empty() {
            name.to_string()
        } else {
            format!("{}.{name}", self.current_module)
        }
    }

    pub(super) fn type_symbol(&self, name: &str) -> String {
        self.type_aliases
            .get(&self.current_module)
            .and_then(|aliases| aliases.get(name))
            .cloned()
            .unwrap_or_else(|| self.symbol(name))
    }

    pub(super) fn qualified_type(&self, ty: &ast::Type) -> ast::Type {
        self.qualified_type_preserving(ty, &BTreeSet::new())
    }

    fn qualified_type_preserving(&self, ty: &ast::Type, generics: &BTreeSet<String>) -> ast::Type {
        match ty {
            ast::Type::Named(path) => ast::Type::Named(ast::ModulePath(
                self.type_symbol(&path.as_dotted())
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
                .constant_value(name)
                .map(ArraySize::Literal)
                // Keep lexical ownership even if a constant is unresolved;
                // never capture an equally named entry-module constant.
                .unwrap_or_else(|| {
                    ArraySize::Param(self.constant_symbol(name).unwrap_or_else(|| name.clone()))
                }),
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
