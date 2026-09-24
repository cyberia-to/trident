//! Explicit Continue/Return values avoid copying a block's continuation.
use super::*;

impl Compiler<'_> {
    pub(super) fn block(&mut self, block: &Block) -> LowerResult {
        self.scopes.push(BTreeMap::new());
        let result = (|| {
            let mut formulas = Vec::new();
            for stmt in &block.stmts {
                formulas.push(self.statement(stmt)?);
            }
            if let Some(e) = &block.tail_expr {
                formulas.push(self.effect(&e.node)?);
            }
            let mut result = keep(nox_axis(1));
            for formula in formulas.into_iter().rev() {
                result = layout::then(formula, result);
            }
            Ok(result)
        })();
        self.scopes.pop();
        result
    }
    fn effect(&mut self, e: &Expr) -> LowerResult {
        Ok(seq(nox_cons(self.expr(e)?, nox_axis(1)), keep(nox_axis(3))))
    }
    fn statement(&mut self, stmt: &Spanned<Stmt>) -> LowerResult {
        let key = (stmt.span.start, stmt.span.end);
        match &stmt.node {
            Stmt::Let {
                pattern, ty, init, ..
            } => {
                let value = self.expr(&init.node)?;
                let ty = ty
                    .as_ref()
                    .map(|t| self.owner.qualified_type(&t.node))
                    .or_else(|| self.ty(&init.node))
                    .unwrap_or(Type::Field);
                let slots = self
                    .function
                    .slots
                    .get(&key)
                    .cloned()
                    .ok_or("missing native binding slots")?;
                match pattern {
                    Pattern::Name(name) => {
                        let set = self.set(slots[0], value)?;
                        self.bind(
                            &name.node,
                            Local {
                                slot: slots[0],
                                ty,
                                range: None,
                            },
                        )?;
                        Ok(keep(set))
                    }
                    Pattern::Tuple(names) => {
                        let temp = *slots.last().ok_or("missing native aggregate temporary")?;
                        let mut result = self.set(temp, value)?;
                        for (i, name) in names.iter().enumerate() {
                            let element = aggregate_element(self.slot(temp), Some(&ty), i as u64)?;
                            result = seq(result, self.set(slots[i], element)?);
                            let item_ty = match &ty {
                                Type::Tuple(ts) => ts.get(i).cloned().unwrap_or(Type::Field),
                                _ => Type::Field,
                            };
                            self.bind(
                                &name.node,
                                Local {
                                    slot: slots[i],
                                    ty: item_ty,
                                    range: None,
                                },
                            )?;
                        }
                        Ok(keep(result))
                    }
                }
            }
            Stmt::TupleAssign { names, value } => {
                let ty = self.ty(&value.node);
                let temp = *self
                    .function
                    .slots
                    .get(&key)
                    .and_then(|s| s.first())
                    .ok_or("missing native tuple temporary")?;
                let value = self.expr(&value.node)?;
                let mut result = self.set(temp, value)?;
                for (i, name) in names.iter().enumerate() {
                    let slot = self
                        .local(&name.node)
                        .ok_or("undefined tuple assignment target")?
                        .slot;
                    let element = aggregate_element(self.slot(temp), ty.as_ref(), i as u64)?;
                    result = seq(result, self.set(slot, element)?);
                }
                Ok(keep(result))
            }
            Stmt::Assign { place, value } => {
                let value = self.assign(&place.node, &value.node)?;
                Ok(keep(value))
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                let condition = self.expr(&cond.node)?;
                let yes = self.block(&then_block.node)?;
                let no = match else_block {
                    Some(b) => self.block(&b.node)?,
                    None => keep(nox_axis(1)),
                };
                Ok(nox_branch(condition, yes, no))
            }
            Stmt::Return(e) => Ok(returned(match e {
                Some(e) => self.expr(&e.node)?,
                None => nox_unit(),
            })),
            Stmt::Expr(e) => self.effect(&e.node),
            Stmt::For {
                var,
                start,
                end,
                bound,
                body,
            } => self.for_loop(key, &var.node, &start.node, &end.node, *bound, &body.node),
            Stmt::Asm { .. } => Err("nox: inline assembly not supported".into()),
            Stmt::Match { .. } => Err("nox: match not yet supported".into()),
            Stmt::Reveal { .. } | Stmt::Seal { .. } => {
                Err("raw ART1 profile forbids event services".into())
            }
        }
    }
}
