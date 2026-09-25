//! Layout information carried across module boundaries into stack lowering.
use super::TIRBuilder;
use crate::ast::{ArraySize, BinOp, Expr, File, Item, Literal, Type};
use std::collections::BTreeSet;

impl TIRBuilder {
    pub fn with_module_types(mut self, modules: &[&File]) -> Self {
        // Resolve when build_file runs, after the builder's final cfg flags.
        self.module_type_files = modules.iter().map(|file| (*file).clone()).collect();
        self
    }

    pub(super) fn prepare_module_types(
        &mut self,
        entry: &File,
    ) -> Result<File, Vec<crate::diagnostic::Diagnostic>> {
        let mut files = std::mem::take(&mut self.module_type_files);
        let index = if let Some(index) = files.iter().position(|f| f.name.node == entry.name.node) {
            files[index] = entry.clone();
            index
        } else {
            files.push(entry.clone());
            files.len() - 1
        };
        let refs: Vec<_> = files.iter().collect();
        let scopes = crate::resolve::scope::scopes(&refs)?;
        let builtins = crate::typecheck::TypeChecker::builtin_return_types(&self.target_config);
        let resolved = crate::typecheck::constants::resolve_modules(
            &refs,
            &self.cfg_flags,
            &self.constant_bindings,
        )?;
        for ((module, scope), constants) in files.iter().zip(&scopes).zip(&resolved) {
            scope.validate_names(module, &refs, &self.cfg_flags, &builtins, constants)?;
        }
        let aliases: Vec<_> = files
            .iter()
            .zip(&scopes)
            .map(|(file, scope)| scope.type_aliases(file, &refs, &self.cfg_flags))
            .collect();
        self.function_aliases
            .extend(scopes[index].function_aliases(&refs, &self.cfg_flags));
        for (i, module) in files.iter_mut().enumerate() {
            let constants = resolved[i].raw_values();
            super::nominal::Nominal {
                aliases: &aliases[i],
                constants: &constants,
                parameters: BTreeSet::new(),
            }
            .file(module);
            if i == index {
                self.constant_bindings = resolved[i].visible.clone();
            }
            for item in &module.items {
                if !self.is_item_cfg_active(&item.node) {
                    continue;
                }
                match &item.node {
                    Item::Struct(def) => {
                        self.struct_types.insert(def.name.node.clone(), def.clone());
                    }
                    Item::Fn(func) => {
                        let ty = func
                            .return_ty
                            .as_ref()
                            .map(|t| t.node.clone())
                            .unwrap_or_else(|| Type::Tuple(Vec::new()));
                        self.fn_return_types
                            .insert(format!("{}.{}", module.name.node, func.name.node), ty);
                    }
                    _ => {}
                }
            }
        }
        Ok(files.remove(index))
    }

    pub(crate) fn qualified_function(&self, name: &str) -> String {
        self.function_aliases
            .get(name)
            .cloned()
            .unwrap_or_else(|| self.qualified_name(name))
    }

    pub(crate) fn qualified_name(&self, name: &str) -> String {
        if let Some((module, tail)) = name.rsplit_once('.') {
            if let Some(full) = self.module_aliases.get(module) {
                return format!("{full}.{tail}");
            }
        }
        name.to_string()
    }

    pub(crate) fn constant_value(&self, name: &str) -> Option<u64> {
        self.constants.get(name).copied()
    }

    /// Checked logical extent, independent of the element's machine width.
    pub(crate) fn array_count(&self, size: &ArraySize) -> Option<u32> {
        u32::try_from(self.array_extent(size)?).ok()
    }

    fn array_extent(&self, size: &ArraySize) -> Option<u64> {
        match size {
            ArraySize::Literal(n) => Some(*n),
            ArraySize::Param(name) => self.constant_value(name),
            ArraySize::Add(left, right) => self
                .array_extent(left)?
                .checked_add(self.array_extent(right)?),
            ArraySize::Mul(left, right) => self
                .array_extent(left)?
                .checked_mul(self.array_extent(right)?),
        }
    }

    pub(crate) fn type_width(&self, ty: &Type) -> u32 {
        match ty {
            Type::Named(path) => self
                .struct_types
                .get(&path.as_dotted())
                .map(|s| s.fields.iter().map(|f| self.type_width(&f.ty.node)).sum())
                .unwrap_or(1),
            Type::Array(inner, n) => self.type_width(inner) * self.array_count(n).unwrap_or(0),
            Type::Tuple(parts) => parts.iter().map(|p| self.type_width(p)).sum(),
            _ => super::layout::resolve_type_width(ty, &self.target_config),
        }
    }

    pub(crate) fn expr_type(&self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Call { path, .. } => {
                let name = path.node.as_dotted();
                let resolved = if self.generic_fn_defs.contains_key(&name) {
                    self.call_resolutions
                        .get(&(
                            self.current_function.clone(),
                            path.span.start,
                            path.span.end,
                        ))
                        .map(|instance| instance.mangled_name())?
                } else {
                    self.qualified_function(&name)
                };
                self.fn_return_types.get(&resolved).cloned()
            }
            Expr::StructInit { path, .. } => Some(Type::Named(path.node.clone())),
            Expr::Var(name) => {
                let mut parts = name.split('.');
                let mut ty = match self.var_types.get(parts.next()?) {
                    Some(ty) => ty.clone(),
                    None if self.constant_value(name).is_some() => {
                        return match self.constant_bindings.get(name)?.ty {
                            crate::types::Ty::Field => Some(Type::Field),
                            crate::types::Ty::U32 => Some(Type::U32),
                            _ => None,
                        };
                    }
                    None => return None,
                };
                for field in parts {
                    ty = self.field_type_offset(&ty, field)?.0;
                }
                Some(ty)
            }
            Expr::Index { expr, .. } => match self.expr_type(&expr.node)? {
                Type::Array(element, _) => Some(*element),
                _ => None,
            },
            Expr::FieldAccess { expr, field } => self
                .field_type_offset(&self.expr_type(&expr.node)?, &field.node)
                .map(|(ty, _)| ty),
            Expr::ArrayInit(elements) => elements
                .first()
                .and_then(|e| self.expr_type(&e.node))
                .or_else(|| elements.is_empty().then_some(Type::Field))
                .map(|ty| Type::Array(Box::new(ty), ArraySize::Literal(elements.len() as u64))),
            Expr::BinOp { op, lhs, .. } => match op {
                BinOp::Eq | BinOp::Lt => Some(Type::Bool),
                BinOp::BitAnd | BinOp::BitXor => Some(Type::U32),
                BinOp::DivMod => Some(Type::Tuple(vec![Type::U32, Type::U32])),
                BinOp::XFieldMul => Some(Type::XField),
                BinOp::Add | BinOp::Mul => self.expr_type(&lhs.node).or(Some(Type::Field)),
            },
            Expr::Literal(Literal::Integer(_)) => Some(Type::Field),
            Expr::Literal(Literal::Bool(_)) => Some(Type::Bool),
            Expr::Tuple(parts) => parts
                .iter()
                .map(|p| self.expr_type(&p.node))
                .collect::<Option<Vec<_>>>()
                .map(Type::Tuple),
        }
    }
}
