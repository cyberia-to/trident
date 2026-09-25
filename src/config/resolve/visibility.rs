//! Validate source names before lowering can see the canonical layout registry.
use super::scope::Scope;
use crate::ast::{self, surface::SurfaceVisitor, ArraySize, File, Item, Type};
use crate::diagnostic::Diagnostic;
use std::collections::{BTreeMap, BTreeSet};

impl Scope {
    pub fn validate_names(
        &self,
        file: &File,
        files: &[&File],
        flags: &BTreeSet<String>,
        builtins: &BTreeMap<String, Type>,
        constants: &crate::typecheck::constants::Resolved,
    ) -> Result<(), Vec<Diagnostic>> {
        let mut guard = Names {
            functions: self.function_aliases(files, flags),
            types: self.type_aliases(file, files, flags),
            missing: BTreeSet::new(),
            constants: constants.visible.keys().cloned().collect(),
            size_params: BTreeSet::new(),
        };
        for name in builtins.keys() {
            guard
                .functions
                .entry(name.clone())
                .or_insert_with(|| name.clone());
        }
        ast::surface::declarations(file, &mut guard);
        for item in &file.items {
            if item.node.cfg_active(flags) {
                guard.size_params = match &item.node {
                    Item::Fn(f) => f.type_params.iter().map(|p| p.node.clone()).collect(),
                    _ => BTreeSet::new(),
                };
                ast::surface::item(&item.node, &mut guard);
            }
        }
        if guard.missing.is_empty() {
            Ok(())
        } else {
            Err(guard
                .missing
                .into_iter()
                .map(|name| {
                    Diagnostic::error(
                        format!("'{name}' is not visible in module '{}'", file.name.node),
                        file.name.span,
                    )
                })
                .collect())
        }
    }
}
struct Names {
    functions: BTreeMap<String, String>,
    types: BTreeMap<String, String>,
    missing: BTreeSet<String>,
    constants: BTreeSet<String>,
    size_params: BTreeSet<String>,
}
impl SurfaceVisitor for Names {
    fn ty(&mut self, ty: &Type) {
        match ty {
            Type::Named(path) if !self.types.contains_key(&path.as_dotted()) => {
                self.missing.insert(path.as_dotted());
            }
            Type::Array(inner, size) => {
                self.ty(inner);
                self.size(size);
            }
            Type::Tuple(parts) => {
                for part in parts {
                    self.ty(part);
                }
            }
            _ => {}
        }
    }
    fn call(&mut self, name: &str) {
        if name.contains('.') && !self.functions.contains_key(name) {
            self.missing.insert(name.into());
        }
    }
}

impl Names {
    fn size(&mut self, size: &ArraySize) {
        match size {
            ArraySize::Param(name)
                if !self.constants.contains(name) && !self.size_params.contains(name) =>
            {
                self.missing.insert(name.clone());
            }
            ArraySize::Add(a, b) | ArraySize::Mul(a, b) => {
                self.size(a);
                self.size(b);
            }
            _ => {}
        }
    }
}
