//! Assertion-failure paths do not contribute a value to a branch join.
use super::TIRBuilder;
use crate::ast::*;
use std::collections::BTreeMap;

impl TIRBuilder {
    fn is_halting_expr(&self, expr: &Expr) -> bool {
        crate::ast::is_false_assert(expr, |name| match self.intrinsic_map.get(name) {
            Some(intrinsic) => intrinsic == "assert",
            None => {
                name == "assert"
                    && !self.fn_return_widths.contains_key(name)
                    && !self.generic_fn_defs.contains_key(name)
            }
        })
    }

    pub(super) fn block_halts(&self, block: &Block) -> bool {
        let mut constants = self.constants.clone();
        for name in self.var_types.keys() {
            shadow_flow_constant(&mut constants, name);
        }
        self.block_halts_with_constants(block, &constants)
    }

    fn block_halts_with_constants(&self, block: &Block, constants: &BTreeMap<String, u64>) -> bool {
        let mut constants = constants.clone();
        for statement in &block.stmts {
            let halts = match &statement.node {
                Stmt::Expr(expr) | Stmt::Return(Some(expr)) => self.is_halting_expr(&expr.node),
                Stmt::If {
                    cond,
                    then_block,
                    else_block,
                } => {
                    let then_halts = self.block_halts_with_constants(&then_block.node, &constants);
                    let else_halts = else_block.as_ref().is_some_and(|other| {
                        self.block_halts_with_constants(&other.node, &constants)
                    });
                    match constant_condition(&cond.node, &constants, &self.target_config.name) {
                        Some(true) => then_halts,
                        Some(false) => else_halts,
                        None => then_halts && else_halts,
                    }
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
                            self.block_halts_with_constants(&arm.body.node, &inner)
                        })
                }
                Stmt::For {
                    var,
                    start,
                    end,
                    bound,
                    body,
                } => {
                    if constant_nonempty_loop(
                        &start.node,
                        &end.node,
                        *bound,
                        &constants,
                        &self.target_config.name,
                    ) {
                        let mut inner = constants.clone();
                        shadow_flow_constant(&mut inner, &var.node);
                        self.block_halts_with_constants(&body.node, &inner)
                    } else {
                        false
                    }
                }
                _ => false,
            };
            if halts {
                return true;
            }
            match &statement.node {
                Stmt::Return(_) => return false,
                Stmt::Let { pattern, .. } => match pattern {
                    Pattern::Name(name) => {
                        shadow_flow_constant(&mut constants, &name.node);
                    }
                    Pattern::Tuple(names) => {
                        for name in names {
                            shadow_flow_constant(&mut constants, &name.node);
                        }
                    }
                },
                _ => {}
            }
        }
        block
            .tail_expr
            .as_ref()
            .is_some_and(|tail| self.is_halting_expr(&tail.node))
    }

    pub(super) fn join_width(
        &mut self,
        left: Option<u32>,
        right: Option<u32>,
        context: &str,
    ) -> Option<u32> {
        match (left, right) {
            (Some(a), Some(b)) if a != b => {
                self.ops.push(crate::tir::TIROp::Comment(format!(
                    "ERROR: {context} have different result widths"
                )));
                Some(0)
            }
            (Some(width), _) | (_, Some(width)) => Some(width),
            (None, None) => None,
        }
    }
}
