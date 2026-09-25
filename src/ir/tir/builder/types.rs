//! Layout information carried across module boundaries into stack lowering.
use super::TIRBuilder;
use crate::ast::{ArraySize, BinOp, Expr, File, Item, Literal, Type};
use std::collections::{BTreeMap, BTreeSet};

impl TIRBuilder {
    pub fn with_module_types(mut self, modules: &[&File]) -> Self {
        // Resolve when build_file runs, after the builder's final cfg flags.
        self.module_type_files = modules.iter().map(|file| (*file).clone()).collect();
        self
    }

    pub(super) fn prepare_module_types(
        &mut self,
        entry: &str,
    ) -> Result<(), Vec<crate::diagnostic::Diagnostic>> {
        let files = std::mem::take(&mut self.module_type_files);
        let resolved = crate::typecheck::constants::resolve_modules(
            &files.iter().collect::<Vec<_>>(),
            &self.cfg_flags,
            &self.constant_bindings,
        )?;
        for (module, bindings) in files.iter().zip(&resolved) {
            let constants = bindings.raw_values();
            if module.name.node == entry {
                // Layout owners' private names never enter the caller's scope.
                self.constant_bindings.extend(
                    bindings
                        .visible
                        .iter()
                        .filter(|(name, _)| !bindings.locals.contains_key(*name))
                        .map(|(name, binding)| (name.clone(), binding.clone())),
                );
            }
            for item in &module.items {
                if !self.is_item_cfg_active(&item.node) {
                    continue;
                }
                match &item.node {
                    Item::Struct(def) => {
                        let mut def = def.clone();
                        for field in &mut def.fields {
                            qualify(
                                &mut field.ty.node,
                                &module.name.node,
                                &constants,
                                &BTreeSet::new(),
                            );
                        }
                        self.struct_types
                            .insert(format!("{}.{}", module.name.node, def.name.node), def);
                    }
                    Item::Fn(func) => {
                        let mut ty = func
                            .return_ty
                            .as_ref()
                            .map(|ty| ty.node.clone())
                            .unwrap_or_else(|| Type::Tuple(Vec::new()));
                        qualify(
                            &mut ty,
                            &module.name.node,
                            &constants,
                            &func.type_params.iter().map(|p| p.node.clone()).collect(),
                        );
                        self.fn_return_types
                            .insert(format!("{}.{}", module.name.node, func.name.node), ty);
                    }
                    _ => {}
                }
            }
        }
        if !files.iter().any(|file| file.name.node == entry) {
            for bindings in &resolved {
                bindings.import_into(&mut self.constant_bindings);
            }
        }
        Ok(())
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
        self.constants
            .get(name)
            .or_else(|| self.constants.get(&self.qualified_name(name)))
            .copied()
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
                .get(&self.qualified_name(&path.0.join(".")))
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

fn qualify(
    ty: &mut Type,
    module: &str,
    constants: &BTreeMap<String, u64>,
    parameters: &BTreeSet<String>,
) {
    match ty {
        Type::Named(path) if path.0.len() == 1 => {
            path.0.insert(0, module.to_string());
        }
        Type::Array(inner, size) => {
            qualify(inner, module, constants, parameters);
            qualify_size(size, constants, parameters);
        }
        Type::Tuple(parts) => {
            for part in parts {
                qualify(part, module, constants, parameters);
            }
        }
        _ => {}
    }
}

fn qualify_size(
    size: &mut ArraySize,
    constants: &BTreeMap<String, u64>,
    parameters: &BTreeSet<String>,
) {
    match size {
        ArraySize::Param(name) if constants.contains_key(name) && !parameters.contains(name) => {
            *size = ArraySize::Literal(constants[name]);
        }
        ArraySize::Add(left, right) | ArraySize::Mul(left, right) => {
            qualify_size(left, constants, parameters);
            qualify_size(right, constants, parameters);
        }
        _ => {}
    }
}
