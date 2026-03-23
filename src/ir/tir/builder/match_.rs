// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Match statement compilation.

use crate::ast::*;
use crate::span::Spanned;
use crate::tir::TIROp;

use super::TIRBuilder;

impl TIRBuilder {
    pub(crate) fn build_match(&mut self, expr: &Spanned<Expr>, arms: &[MatchArm]) {
        self.build_expr(&expr.node);
        if let Some(top) = self.stack.last_mut() {
            top.name = Some("__match_scrutinee".to_string());
        }

        let mut deferred_subs: Vec<(String, Block, bool)> = Vec::new();

        for arm in arms {
            match &arm.pattern.node {
                MatchPattern::Literal(lit) => {
                    let _arm_label = self.fresh_label("match_arm");
                    let _rest_label = self.fresh_label("match_rest");

                    let depth = self.stack.access_var("__match_scrutinee");
                    self.flush_stack_effects();
                    self.ops.push(TIROp::Dup(depth));

                    match lit {
                        Literal::Integer(n) => self.ops.push(TIROp::Push(*n)),
                        Literal::Bool(b) => self.ops.push(TIROp::Push(if *b { 1 } else { 0 })),
                    }

                    self.ops.push(TIROp::Eq);

                    let mut arm_stmts = vec![Spanned::new(
                        Stmt::Asm {
                            body: "pop 1".to_string(),
                            effect: -1,
                            target: None,
                        },
                        arm.body.span,
                    )];
                    arm_stmts.extend(arm.body.node.stmts.clone());

                    let arm_block = Block {
                        stmts: arm_stmts,
                        tail_expr: arm.body.node.tail_expr.clone(),
                    };

                    let rest_block = Block {
                        stmts: Vec::new(),
                        tail_expr: None,
                    };

                    let saved = self.stack.save_state();

                    let then_body = self.build_deferred_arm_ir(&arm_block, true);
                    self.stack.restore_state(saved.clone());

                    let else_body = self.build_deferred_arm_ir(&rest_block, false);
                    self.stack.restore_state(saved);

                    self.ops.push(TIROp::IfElse {
                        then_body,
                        else_body,
                    });
                }

                MatchPattern::Wildcard => {
                    let w_label = self.fresh_label("match_wild");
                    self.ops.push(TIROp::Call(w_label.clone()));

                    let mut arm_stmts = vec![Spanned::new(
                        Stmt::Asm {
                            body: "pop 1".to_string(),
                            effect: -1,
                            target: None,
                        },
                        arm.body.span,
                    )];
                    arm_stmts.extend(arm.body.node.stmts.clone());
                    deferred_subs.push((
                        w_label,
                        Block {
                            stmts: arm_stmts,
                            tail_expr: arm.body.node.tail_expr.clone(),
                        },
                        false,
                    ));
                }

                MatchPattern::Struct { name, fields } => {
                    let s_label = self.fresh_label("match_struct");
                    self.ops.push(TIROp::Call(s_label.clone()));

                    let mut arm_stmts: Vec<Spanned<Stmt>> = Vec::new();

                    arm_stmts.push(Spanned::new(
                        Stmt::Asm {
                            body: "pop 1".to_string(),
                            effect: -1,
                            target: None,
                        },
                        arm.body.span,
                    ));

                    if let Some(sdef) = self.struct_types.get(&name.node).cloned() {
                        for spf in fields {
                            let field_name = &spf.field_name.node;
                            let access_expr = Expr::FieldAccess {
                                expr: Box::new(expr.clone()),
                                field: spf.field_name.clone(),
                            };
                            let access_spanned = Spanned::new(access_expr, spf.field_name.span);

                            match &spf.pattern.node {
                                FieldPattern::Binding(var_name) => {
                                    let field_ty = sdef
                                        .fields
                                        .iter()
                                        .find(|f| f.name.node == *field_name)
                                        .map(|f| f.ty.clone());
                                    arm_stmts.push(Spanned::new(
                                        Stmt::Let {
                                            mutable: false,
                                            pattern: Pattern::Name(Spanned::new(
                                                var_name.clone(),
                                                spf.pattern.span,
                                            )),
                                            ty: field_ty,
                                            init: access_spanned,
                                        },
                                        spf.field_name.span,
                                    ));
                                }
                                FieldPattern::Literal(lit) => {
                                    let lit_expr =
                                        Spanned::new(Expr::Literal(lit.clone()), spf.pattern.span);
                                    let eq_expr = Spanned::new(
                                        Expr::BinOp {
                                            op: BinOp::Eq,
                                            lhs: Box::new(access_spanned),
                                            rhs: Box::new(lit_expr),
                                        },
                                        spf.pattern.span,
                                    );
                                    arm_stmts.push(Spanned::new(
                                        Stmt::Expr(Spanned::new(
                                            Expr::Call {
                                                path: Spanned::new(
                                                    ModulePath::single("assert".to_string()),
                                                    spf.pattern.span,
                                                ),
                                                generic_args: vec![],
                                                args: vec![eq_expr],
                                            },
                                            spf.pattern.span,
                                        )),
                                        spf.pattern.span,
                                    ));
                                }
                                FieldPattern::Wildcard => {}
                            }
                        }
                    }

                    arm_stmts.extend(arm.body.node.stmts.clone());
                    deferred_subs.push((
                        s_label,
                        Block {
                            stmts: arm_stmts,
                            tail_expr: arm.body.node.tail_expr.clone(),
                        },
                        false,
                    ));
                }
            }
        }

        // Pop the scrutinee after match completes.
        self.stack.pop();
        self.ops.push(TIROp::Pop(1));

        // Emit deferred subroutines inline.
        for (label, block, _is_literal) in deferred_subs {
            self.ops.push(TIROp::FnStart(label));
            let saved = self.stack.save_state();
            self.stack.clear();
            self.build_block(&block);
            self.stack.restore_state(saved);
            self.ops.push(TIROp::Return);
            self.ops.push(TIROp::FnEnd);
        }
    }

    /// Build a deferred match arm body into IR.
    pub(crate) fn build_deferred_arm_ir(&mut self, block: &Block, clears_flag: bool) -> Vec<TIROp> {
        let saved_ops = std::mem::take(&mut self.ops);
        if clears_flag {
            self.ops.push(TIROp::Push(0));
        }
        self.build_block(block);
        self.ops.push(TIROp::Return);

        let nested = std::mem::take(&mut self.ops);
        self.ops = saved_ops;
        nested
    }
}
