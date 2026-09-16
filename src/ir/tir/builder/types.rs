//! Layout information carried across module boundaries into stack lowering.
use super::TIRBuilder;
use crate::ast::{ArraySize, BinOp, Expr, File, Item, Literal, Type};

impl TIRBuilder {
    pub fn with_module_types(mut self, modules: &[&File]) -> Self {
        for module in modules {
            for item in &module.items {
                if !self.is_item_cfg_active(&item.node) {
                    continue;
                }
                match &item.node {
                    Item::Struct(def) => {
                        let mut def = def.clone();
                        for field in &mut def.fields {
                            qualify(&mut field.ty.node, &module.name.node);
                        }
                        self.struct_types
                            .insert(format!("{}.{}", module.name.node, def.name.node), def);
                    }
                    Item::Fn(func) => {
                        if let Some(ty) = &func.return_ty {
                            let mut ty = ty.node.clone();
                            qualify(&mut ty, &module.name.node);
                            self.fn_return_types
                                .insert(format!("{}.{}", module.name.node, func.name.node), ty);
                        }
                    }
                    _ => {}
                }
            }
        }
        self
    }

    pub(crate) fn qualified_name(&self, name: &str) -> String {
        if let Some((module, tail)) = name.rsplit_once('.') {
            if let Some(full) = self.module_aliases.get(module) {
                return format!("{full}.{tail}");
            }
        }
        name.to_string()
    }

    pub(crate) fn type_width(&self, ty: &Type) -> u32 {
        match ty {
            Type::Named(path) => self
                .struct_types
                .get(&self.qualified_name(&path.0.join(".")))
                .map(|s| s.fields.iter().map(|f| self.type_width(&f.ty.node)).sum())
                .unwrap_or(1),
            Type::Array(inner, n) => self.type_width(inner) * n.eval(&self.current_subs) as u32,
            Type::Tuple(parts) => parts.iter().map(|p| self.type_width(p)).sum(),
            _ => super::layout::resolve_type_width(ty, &self.target_config),
        }
    }

    pub(crate) fn expr_type(&self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Call { path, .. } => self
                .fn_return_types
                .get(&self.qualified_name(&path.node.0.join(".")))
                .cloned(),
            Expr::StructInit { path, .. } => Some(Type::Named(path.node.clone())),
            Expr::Var(name) => {
                let mut parts = name.split('.');
                let mut ty = self.var_types.get(parts.next()?)?.clone();
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

fn qualify(ty: &mut Type, module: &str) {
    match ty {
        Type::Named(path) if path.0.len() == 1 => {
            path.0.insert(0, module.to_string());
        }
        Type::Array(inner, _) => qualify(inner, module),
        Type::Tuple(parts) => {
            for part in parts {
                qualify(part, module);
            }
        }
        _ => {}
    }
}
