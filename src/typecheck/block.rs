//! Statement type checking: check_fn, check_block, check_stmt, check_event_stmt, check_place.

use crate::ast::*;
use crate::span::{Span, Spanned};
use crate::types::Ty;

use super::TypeChecker;

impl TypeChecker {
    pub(super) fn check_fn(&mut self, func: &FnDef) {
        if func.body.is_none() {
            return; // intrinsic, no body to check
        }
        if !func.type_params.is_empty() {
            return; // generic — body checked per monomorphized instance
        }

        // Validate #[test] functions: no parameters, no return type, not generic.
        if func.is_test {
            if !func.params.is_empty() {
                self.error(
                    format!(
                        "#[test] function '{}' must have no parameters",
                        func.name.node
                    ),
                    func.name.span,
                );
            }
            if func.return_ty.is_some() {
                self.error(
                    format!(
                        "#[test] function '{}' must not have a return type",
                        func.name.node
                    ),
                    func.name.span,
                );
            }
        }

        let prev_pure = self.in_pure_fn;
        self.in_pure_fn = func.is_pure;

        self.push_scope();

        // Bind parameters
        for param in &func.params {
            let ty = self.resolve_type(&param.ty.node);
            self.define_var(&param.name.node, ty, false);
        }

        let body = func.body.as_ref().expect("guarded by is_none check above");
        self.check_block(&body.node);

        self.pop_scope();
        self.in_pure_fn = prev_pure;
    }

    pub(super) fn check_block(&mut self, block: &Block) -> Ty {
        self.push_scope();
        let mut terminated = false;
        for stmt in &block.stmts {
            if terminated {
                self.error_with_help(
                    "unreachable code after return statement".to_string(),
                    stmt.span,
                    "remove this code or move it before the return".to_string(),
                );
                break;
            }
            self.check_stmt(&stmt.node, stmt.span);
            if self.is_terminating_stmt(&stmt.node) {
                terminated = true;
            }
        }
        if terminated {
            if let Some(tail) = &block.tail_expr {
                self.error_with_help(
                    "unreachable tail expression after return".to_string(),
                    tail.span,
                    "remove this expression or move it before the return".to_string(),
                );
            }
        }
        let ty = if let Some(tail) = &block.tail_expr {
            self.check_expr(&tail.node, tail.span)
        } else {
            Ty::Unit
        };
        self.pop_scope();
        ty
    }

    pub(super) fn is_terminating_stmt(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Return(_) => true,
            // assert(false) is an unconditional halt
            Stmt::Expr(expr) => {
                if let Expr::Call { path, args, .. } = &expr.node {
                    let name = path.node.as_dotted();
                    if (name == "assert" || name == "assert.is_true") && args.len() == 1 {
                        if let Expr::Literal(Literal::Bool(false)) = &args[0].node {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub(super) fn check_event_stmt(
        &mut self,
        event_name: &Spanned<String>,
        fields: &[(Spanned<String>, Spanned<Expr>)],
    ) {
        let Some(event_fields) = self.events.get(&event_name.node).cloned() else {
            self.error(
                format!("undefined event '{}'", event_name.node),
                event_name.span,
            );
            return;
        };

        // Check all declared fields are provided
        for (def_name, _def_ty) in &event_fields {
            if !fields.iter().any(|(n, _)| n.node == *def_name) {
                self.error(
                    format!(
                        "missing field '{}' in event '{}'",
                        def_name, event_name.node
                    ),
                    event_name.span,
                );
            }
        }

        // Check provided fields exist and have correct types
        for (name, val) in fields {
            if let Some((_def_name, def_ty)) = event_fields.iter().find(|(n, _)| *n == name.node) {
                let val_ty = self.check_expr(&val.node, val.span);
                if val_ty != *def_ty {
                    self.error(
                        format!(
                            "event field '{}': expected {} but got {}",
                            name.node,
                            def_ty.display(),
                            val_ty.display()
                        ),
                        val.span,
                    );
                }
            } else {
                self.error(
                    format!(
                        "unknown field '{}' in event '{}'",
                        name.node, event_name.node
                    ),
                    name.span,
                );
            }
        }
    }

    pub(super) fn check_place(&mut self, place: &Place, _span: Span) -> (Ty, bool) {
        match place {
            Place::Var(name) => {
                if let Some(info) = self.lookup_var(name) {
                    return (info.ty.clone(), info.mutable);
                }
                // Dotted name = struct field assignment (`p.x`, `p.q.r`). The
                // mutability comes from the base variable; the type is that of
                // the addressed field.
                if let Some(base) = name.split('.').next() {
                    if name.contains('.') {
                        if let Some(info) = self.lookup_var(base) {
                            let is_mut = info.mutable;
                            if let Some(ty) = self.resolve_nested_field_access(name, _span) {
                                return (ty, is_mut);
                            }
                        }
                    }
                }
                (Ty::Field, false)
            }
            Place::FieldAccess(inner, field) => {
                let (inner_ty, is_mut) = self.check_place(&inner.node, inner.span);
                if let Ty::Struct(sty) = &inner_ty {
                    if let Some((field_ty, _, _)) = sty.field_offset(&field.node) {
                        (field_ty, is_mut)
                    } else {
                        (Ty::Field, false)
                    }
                } else {
                    (Ty::Field, false)
                }
            }
            Place::Index(inner, index) => {
                let (inner_ty, is_mut) = self.check_place(&inner.node, inner.span);
                self.check_expr(&index.node, index.span);
                if let Ty::Array(elem_ty, _) = &inner_ty {
                    (*elem_ty.clone(), is_mut)
                } else {
                    (Ty::Field, false)
                }
            }
        }
    }
}
