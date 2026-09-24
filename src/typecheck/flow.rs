//! Conservative return and assertion-failure analysis with lexical constants.
use super::TypeChecker;
use crate::ast::*;
use std::collections::BTreeMap;

impl TypeChecker {
    pub(super) fn is_halting_expr(&self, expr: &Expr) -> bool {
        crate::ast::is_false_assert(expr, |name| {
            !self.generic_fns.contains_key(name)
                && self
                    .functions
                    .get(name)
                    .is_some_and(|sig| sig.intrinsic.as_deref() == Some("assert"))
        })
    }

    pub(super) fn shadow_constants(statement: &Stmt, visible: &mut BTreeMap<String, u64>) {
        if let Stmt::Let { pattern, .. } = statement {
            match pattern {
                Pattern::Name(name) => {
                    shadow_flow_constant(visible, &name.node);
                }
                Pattern::Tuple(names) => {
                    for name in names {
                        shadow_flow_constant(visible, &name.node);
                    }
                }
            }
        }
    }

    pub(super) fn block_always_returns(
        &self,
        body: &Block,
        constants: &BTreeMap<String, u64>,
    ) -> bool {
        let mut visible = constants.clone();
        for statement in &body.stmts {
            if self.statement_always_returns(&statement.node, &visible) {
                return true;
            }
            Self::shadow_constants(&statement.node, &mut visible);
        }
        body.tail_expr
            .as_ref()
            .is_some_and(|tail| self.is_halting_expr(&tail.node))
    }

    pub(super) fn statement_always_returns(
        &self,
        statement: &Stmt,
        constants: &BTreeMap<String, u64>,
    ) -> bool {
        match statement {
            Stmt::Return(_) => true,
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                let taken = constant_condition(&cond.node, constants, &self.target_config.name);
                let then_returns = self.block_always_returns(&then_block.node, constants);
                let else_returns = else_block
                    .as_ref()
                    .is_some_and(|b| self.block_always_returns(&b.node, constants));
                match taken {
                    Some(true) => then_returns,
                    Some(false) => else_returns,
                    None => then_returns && else_returns,
                }
            }
            Stmt::For {
                var,
                start,
                end,
                bound,
                body,
            } => {
                if !constant_nonempty_loop(
                    &start.node,
                    &end.node,
                    *bound,
                    constants,
                    &self.target_config.name,
                ) {
                    return false;
                }
                let mut inner = constants.clone();
                shadow_flow_constant(&mut inner, &var.node);
                self.block_always_returns(&body.node, &inner)
            }
            Stmt::Match { arms, .. } => {
                !arms.is_empty()
                    && arms.iter().all(|arm| {
                        let mut inner = constants.clone();
                        if let MatchPattern::Struct { fields, .. } = &arm.pattern.node {
                            for field in fields {
                                if let FieldPattern::Binding(name) = &field.pattern.node {
                                    shadow_flow_constant(&mut inner, name);
                                }
                            }
                        }
                        self.block_always_returns(&arm.body.node, &inner)
                    })
            }
            Stmt::Expr(expr) => self.is_halting_expr(&expr.node),
            _ => false,
        }
    }
}
