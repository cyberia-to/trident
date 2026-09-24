//! Pure expressions over complete tree values and stable frame slots.
use super::*;

impl Compiler<'_> {
    pub(super) fn ty(&self, e: &Expr) -> Option<Type> {
        Some(match e {
            Expr::Literal(Literal::Integer(_)) => Type::Field,
            Expr::Literal(Literal::Bool(_)) => Type::Bool,
            Expr::Var(n) => self
                .dotted(n)
                .ok()
                .map(|p| p.1)
                .or_else(|| self.constant(e).map(|_| Type::Field))?,
            Expr::StructInit { path, .. } => {
                self.owner.qualified_type(&Type::Named(path.node.clone()))
            }
            Expr::ArrayInit(es) => Type::Array(
                Box::new(
                    es.first()
                        .and_then(|e| self.ty(&e.node))
                        .unwrap_or(Type::Field),
                ),
                ast::ArraySize::Literal(es.len() as u64),
            ),
            Expr::Tuple(es) => Type::Tuple(
                es.iter()
                    .map(|e| self.ty(&e.node).unwrap_or(Type::Field))
                    .collect(),
            ),
            Expr::FieldAccess { expr, field } => {
                self.owner
                    .field_index(&self.ty(&expr.node)?, &field.node)
                    .ok()?
                    .1
            }
            Expr::Index { expr, .. } => match self.ty(&expr.node)? {
                Type::Array(t, _) => *t,
                Type::Digest => Type::Field,
                _ => return None,
            },
            Expr::BinOp { op, lhs, .. } => match op {
                BinOp::Eq | BinOp::Lt => Type::Bool,
                _ => self.ty(&lhs.node)?,
            },
            Expr::Call { path, .. } => {
                let source = path.node.as_dotted();
                if let Some(f) = self.owner.fns.get(&self.owner.symbol(&source)) {
                    return f.return_ty.as_ref().map(|t| t.node.clone());
                }
                if let Some(ty) = noun::return_type(&source) {
                    return Some(ty);
                }
                match source.as_str() {
                    "as_u32" => Type::U32,
                    "hash" | "std.crypto.hash" => Type::Digest,
                    "assert" | "assert_eq" | "assert_digest" => return None,
                    _ => Type::Field,
                }
            }
        })
    }

    pub(super) fn expr(&mut self, expr: &Expr) -> LowerResult {
        match expr {
            Expr::Literal(Literal::Integer(n)) => {
                Ok(nox_quote(Noun::atom(nebu::Goldilocks::new(*n).as_u64())))
            }
            Expr::Literal(Literal::Bool(b)) => Ok(nox_quote(Noun::atom(u64::from(!*b)))),
            Expr::Var(n) => {
                if let Some(local) = self.local(n) {
                    return Ok(self.slot(local.slot));
                }
                if let Some(n) = self.constant(expr) {
                    return Ok(nox_quote(Noun::atom(nebu::Goldilocks::new(n).as_u64())));
                }
                self.dotted(n).map(|p| p.0)
            }
            Expr::BinOp { op, lhs, rhs } => {
                let a = self.expr(&lhs.node)?;
                let b = self.expr(&rhs.node)?;
                Ok(match op {
                    BinOp::Add => nox_add(a, b),
                    BinOp::Mul => nox_mul(a, b),
                    BinOp::Eq => nox_eq(a, b),
                    BinOp::Lt => nox_lt(a, b),
                    BinOp::BitAnd => nox_and(a, b),
                    BinOp::BitXor => nox_xor(a, b),
                    BinOp::DivMod | BinOp::XFieldMul => {
                        return Err(
                            "native divmod/extension-field multiplication is not implemented"
                                .into(),
                        )
                    }
                })
            }
            Expr::Call { path, args, .. } => self.call(&path.node.as_dotted(), args),
            Expr::FieldAccess { expr, field } => {
                let ty = self.ty(&expr.node).ok_or("unresolved native field type")?;
                let (i, _) = self.owner.field_index(&ty, &field.node)?;
                elem_access(self.expr(&expr.node)?, i as u64)
            }
            Expr::Index { expr, index } => {
                let ty = self.ty(&expr.node).ok_or("unresolved native index type")?;
                let base = self.expr(&expr.node)?;
                let i = self.expr(&index.node)?;
                self.read_index(base, i, &ty)
            }
            Expr::StructInit { path, fields } => {
                let name = self.owner.symbol(&path.node.as_dotted());
                let layout = self
                    .owner
                    .structs
                    .get(&name)
                    .cloned()
                    .ok_or("unresolved native struct")?;
                let mut values = Vec::new();
                for (name, _) in layout {
                    let (_, expr) = fields
                        .iter()
                        .find(|(n, _)| n.node == name)
                        .ok_or("missing native struct field")?;
                    values.push(self.expr(&expr.node)?);
                }
                Ok(cons_list(values))
            }
            Expr::Tuple(es) | Expr::ArrayInit(es) => {
                let values = es
                    .iter()
                    .map(|e| self.expr(&e.node))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(cons_list(values))
            }
        }
    }
}
