//! Layout information carried across module boundaries into stack lowering.
use super::TIRBuilder;
use crate::ast::{ArraySize, BinOp, Expr, File, Item, Literal, Type};
use std::collections::{BTreeMap, BTreeSet};

impl TIRBuilder {
    pub fn with_module_types(mut self, modules: &[&File]) -> Self {
        for module in modules {
            let constants: BTreeMap<_, _> = module
                .items
                .iter()
                .filter_map(|item| {
                    if !self.is_item_cfg_active(&item.node) {
                        return None;
                    }
                    if let Item::Const(def) = &item.node {
                        if let Expr::Literal(Literal::Integer(value)) = def.value.node {
                            return Some((def.name.node.clone(), value));
                        }
                    }
                    None
                })
                .collect();
            for (name, value) in &constants {
                self.constants
                    .insert(format!("{}.{}", module.name.node, name), *value);
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
        self
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
            ArraySize::Param(name) => self
                .current_subs
                .get(name)
                .copied()
                .or_else(|| self.constant_value(name)),
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
            Expr::Call { path, .. } => self
                .fn_return_types
                .get(&self.qualified_function(&path.node.0.join(".")))
                .cloned(),
            Expr::StructInit { path, .. } => Some(Type::Named(path.node.clone())),
            Expr::Var(name) => {
                let mut parts = name.split('.');
                let mut ty = match self.var_types.get(parts.next()?) {
                    Some(ty) => ty.clone(),
                    None if self.constant_value(name).is_some() => return Some(Type::Field),
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
            qualify_size(size, module, constants, parameters);
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
    module: &str,
    constants: &BTreeMap<String, u64>,
    parameters: &BTreeSet<String>,
) {
    match size {
        ArraySize::Param(name) if constants.contains_key(name) && !parameters.contains(name) => {
            *name = format!("{module}.{name}");
        }
        ArraySize::Add(left, right) | ArraySize::Mul(left, right) => {
            qualify_size(left, module, constants, parameters);
            qualify_size(right, module, constants, parameters);
        }
        _ => {}
    }
}
