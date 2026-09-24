//! First-match selection with a single evaluated scrutinee and scoped bindings.
use super::TIRBuilder;
use crate::ast::*;
use crate::span::Spanned;
use crate::tir::TIROp;

impl TIRBuilder {
    pub(crate) fn build_match(&mut self, expr: &Spanned<Expr>, arms: &[MatchArm]) {
        let types = self.var_types.clone();
        let layouts = self.struct_layouts.clone();
        let ty = self.expr_type(&expr.node);
        self.build_expr(&expr.node);
        let name = self.fresh_label("__match_value");
        let width = if let Some(top) = self.stack.last_mut() {
            top.name = Some(name.clone());
            top.width
        } else {
            return;
        };
        if let Some(ty) = ty {
            self.var_types.insert(name.clone(), ty.clone());
            self.register_struct_layout_from_type(&name, &ty);
        }
        let (body, result_width) = self.build_match_arms(&name, arms);
        self.ops.extend(body);
        let result_width = result_width.unwrap_or(0);
        Self::append_branch_cleanup(&mut self.ops, width + result_width, 0, result_width);
        self.stack.pop(); // arm construction restores its caller model
        self.stack.push_temp(result_width);
        self.var_types = types;
        self.struct_layouts = layouts;
    }

    fn build_match_arms(&mut self, name: &str, arms: &[MatchArm]) -> (Vec<TIROp>, Option<u32>) {
        let Some((arm, rest)) = arms.split_first() else {
            // Impossible fallthrough fails explicitly.
            return (vec![TIROp::Push(0), TIROp::Assert(1)], None);
        };
        let saved_ops = std::mem::take(&mut self.ops);
        let saved = self.stack.save_state();
        let types = self.var_types.clone();
        let layouts = self.struct_layouts.clone();
        let pre_depth = self.stack.stack_depth();
        let mut bindings = Vec::new();
        let mut comparisons = Vec::new();
        let span = arm.pattern.span;
        let scrutinee = Spanned::new(Expr::Var(name.into()), span);
        match &arm.pattern.node {
            MatchPattern::Literal(lit) => comparisons.push((scrutinee.clone(), lit.clone())),
            MatchPattern::Wildcard => {}
            MatchPattern::Struct {
                name: struct_name,
                fields,
            } => {
                for field in fields {
                    let access = Spanned::new(
                        Expr::FieldAccess {
                            expr: Box::new(scrutinee.clone()),
                            field: field.field_name.clone(),
                        },
                        span,
                    );
                    match &field.pattern.node {
                        FieldPattern::Binding(binding) => {
                            let ty = self
                                .struct_types
                                .get(&self.qualified_name(&struct_name.node))
                                .and_then(|def| {
                                    def.fields
                                        .iter()
                                        .find(|f| f.name.node == field.field_name.node)
                                })
                                .map(|f| f.ty.clone());
                            bindings.push(Spanned::new(
                                Stmt::Let {
                                    mutable: false,
                                    pattern: Pattern::Name(Spanned::new(binding.clone(), span)),
                                    ty,
                                    init: access,
                                },
                                span,
                            ));
                        }
                        FieldPattern::Literal(lit) => comparisons.push((access, lit.clone())),
                        FieldPattern::Wildcard => {}
                    }
                }
            }
        }
        for (index, (value, literal)) in comparisons.iter().enumerate() {
            self.build_expr(&Expr::BinOp {
                op: BinOp::Eq,
                lhs: Box::new(value.clone()),
                rhs: Box::new(Spanned::new(Expr::Literal(literal.clone()), span)),
            });
            if index > 0 {
                self.ops.push(TIROp::Mul); // conjunction of canonical equality bits
                self.stack.pop();
                self.stack.pop();
                self.stack.push_temp(1);
            }
        }
        let conditional = !comparisons.is_empty();
        if conditional {
            self.stack.pop();
        }
        bindings.extend(arm.body.node.stmts.clone());
        let block = Block {
            stmts: bindings,
            tail_expr: arm.body.node.tail_expr.clone(),
        };
        let (mut then_body, then_width) = self.build_value_block_as_ir(&block);
        Self::append_branch_cleanup(
            &mut then_body,
            self.stack.stack_depth(),
            pre_depth,
            then_width.unwrap_or(0),
        );
        self.stack.restore_state(saved.clone());
        self.var_types = types.clone();
        self.struct_layouts = layouts.clone();
        let result_width = if conditional {
            let (else_body, else_width) = self.build_match_arms(name, rest);
            let width = self.join_width(then_width, else_width, "match arms");
            self.ops.push(TIROp::IfElse {
                then_body,
                else_body,
            });
            width
        } else {
            self.ops.extend(then_body);
            then_width
        };
        self.stack.restore_state(saved);
        self.var_types = types;
        self.struct_layouts = layouts;
        let body = std::mem::replace(&mut self.ops, saved_ops);
        (body, result_width)
    }
}
