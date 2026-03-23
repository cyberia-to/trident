// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Statement type checking: check_fn, check_block, check_stmt, check_event_stmt, check_place.

use crate::ast::*;
use crate::span::Span;
use crate::types::Ty;

use super::TypeChecker;

impl TypeChecker {
    pub(super) fn check_stmt(&mut self, stmt: &Stmt, _span: Span) {
        match stmt {
            Stmt::Let {
                mutable,
                pattern,
                ty,
                init,
            } => {
                let init_ty = self.check_expr(&init.node, init.span);
                let resolved_ty = if let Some(declared_ty) = ty {
                    let expected = self.resolve_type(&declared_ty.node);
                    if expected != init_ty {
                        self.error(
                            format!(
                                "type mismatch: declared {} but expression has type {}",
                                expected.display(),
                                init_ty.display()
                            ),
                            init.span,
                        );
                    }
                    expected
                } else {
                    init_ty
                };

                match pattern {
                    Pattern::Name(name) => {
                        self.define_var(&name.node, resolved_ty.clone(), *mutable);
                        // Track U32-proven variables for H0003:
                        // When as_u32(x) or split(x) is called, the INPUT x
                        // has been range-checked. Mark x as proven so a
                        // subsequent as_u32(x) is flagged as redundant.
                        if let Expr::Call { path, args, .. } = &init.node {
                            let call_name = path.node.as_dotted();
                            let base = call_name.rsplit('.').next().unwrap_or(&call_name);
                            if (base == "as_u32" || base == "split") && !args.is_empty() {
                                if let Expr::Var(arg_name) = &args[0].node {
                                    self.u32_proven.insert(arg_name.clone());
                                }
                            }
                        }
                    }
                    Pattern::Tuple(names) => {
                        // Destructure: type must be a tuple or Digest
                        if let Ty::Tuple(elem_tys) = &resolved_ty {
                            if names.len() != elem_tys.len() {
                                self.error(
                                    format!(
                                        "tuple destructuring: expected {} elements, got {} names",
                                        elem_tys.len(),
                                        names.len()
                                    ),
                                    init.span,
                                );
                            }
                            for (i, name) in names.iter().enumerate() {
                                if name.node != "_" {
                                    let ty = elem_tys.get(i).cloned().unwrap_or(Ty::Field);
                                    self.define_var(&name.node, ty, *mutable);
                                }
                            }
                        } else if matches!(resolved_ty, Ty::Digest(_)) {
                            // Digest decomposition: let (f0, f1, ...) = digest
                            let dw = resolved_ty.width() as usize;
                            if names.len() != dw {
                                self.error(
                                    format!(
                                        "digest destructuring requires exactly {} names, got {}",
                                        dw,
                                        names.len()
                                    ),
                                    init.span,
                                );
                            }
                            for name in names.iter() {
                                if name.node != "_" {
                                    self.define_var(&name.node, Ty::Field, *mutable);
                                }
                            }
                        } else {
                            self.error(
                                format!(
                                    "cannot destructure non-tuple type {}",
                                    resolved_ty.display()
                                ),
                                init.span,
                            );
                        }
                    }
                }
            }
            Stmt::Assign { place, value } => {
                let (place_ty, is_mut) = self.check_place(&place.node, place.span);
                if !is_mut {
                    self.error_with_help(
                        "cannot assign to immutable variable".to_string(),
                        place.span,
                        "declare the variable with `let mut` to make it mutable".to_string(),
                    );
                }
                let val_ty = self.check_expr(&value.node, value.span);
                if place_ty != val_ty {
                    self.error(
                        format!(
                            "type mismatch in assignment: expected {} but got {}",
                            place_ty.display(),
                            val_ty.display()
                        ),
                        value.span,
                    );
                }
                // Invalidate U32-proven status on reassignment
                if let Place::Var(name) = &place.node {
                    self.u32_proven.remove(name);
                }
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                let cond_ty = self.check_expr(&cond.node, cond.span);
                if cond_ty != Ty::Bool && cond_ty != Ty::Field {
                    self.error(
                        format!(
                            "if condition must be Bool or Field, got {}",
                            cond_ty.display()
                        ),
                        cond.span,
                    );
                }
                self.check_block(&then_block.node);
                if let Some(else_blk) = else_block {
                    self.check_block(&else_blk.node);
                }
            }
            Stmt::For {
                var,
                start,
                end,
                bound,
                body,
            } => {
                let _start_ty = self.check_expr(&start.node, start.span);
                let _end_ty = self.check_expr(&end.node, end.span);

                // Check that start is a constant 0 or Field/U32
                // end must be a constant or have bounded annotation
                if bound.is_none() {
                    // end must be a compile-time constant
                    if !self.is_constant_expr(&end.node) {
                        self.error_with_help(
                            "loop end must be a compile-time constant, or annotated with a bound".to_string(),
                            end.span,
                            "use a literal like `for i in 0..10 { }` or add a bound: `for i in 0..n bounded 100 { }`".to_string(),
                        );
                    }
                }

                self.push_scope();
                if var.node != "_" {
                    self.define_var(&var.node, Ty::U32, false);
                }
                self.check_block(&body.node);
                self.pop_scope();
            }
            Stmt::TupleAssign { names, value } => {
                let val_ty = self.check_expr(&value.node, value.span);
                let valid = if let Ty::Tuple(elem_tys) = &val_ty {
                    if names.len() != elem_tys.len() {
                        self.error(
                            format!(
                                "tuple assignment: expected {} elements, got {} names",
                                elem_tys.len(),
                                names.len()
                            ),
                            value.span,
                        );
                    }
                    true
                } else if matches!(val_ty, Ty::Digest(_)) {
                    let dw = val_ty.width() as usize;
                    if names.len() != dw {
                        self.error(
                            format!(
                                "Digest destructuring requires exactly {} names, got {}",
                                dw,
                                names.len()
                            ),
                            value.span,
                        );
                    }
                    true
                } else {
                    false
                };
                if valid {
                    for name in names {
                        if let Some(info) = self.lookup_var(&name.node) {
                            if !info.mutable {
                                self.error_with_help(
                                    format!("cannot assign to immutable variable '{}'", name.node),
                                    name.span,
                                    "declare the variable with `let mut` to make it mutable"
                                        .to_string(),
                                );
                            }
                        }
                    }
                } else {
                    self.error(
                        format!(
                            "cannot tuple-assign from non-tuple type {}",
                            val_ty.display()
                        ),
                        value.span,
                    );
                }
            }
            Stmt::Expr(expr) => {
                self.check_expr(&expr.node, expr.span);
            }
            Stmt::Return(value) => {
                if let Some(val) = value {
                    self.check_expr(&val.node, val.span);
                }
            }
            Stmt::Reveal { event_name, fields } | Stmt::Seal { event_name, fields } => {
                if self.in_pure_fn {
                    let kind = if matches!(stmt, Stmt::Reveal { .. }) {
                        "reveal"
                    } else {
                        "seal"
                    };
                    self.error(
                        format!("#[pure] function cannot use '{}' (I/O side effect)", kind),
                        _span,
                    );
                }
                self.check_event_stmt(event_name, fields);
            }
            Stmt::Asm { target, .. } => {
                // Warn if asm block is tagged for a different target
                if let Some(tag) = target {
                    if tag != &self.target_config.name {
                        self.warning(
                            format!(
                                "asm block tagged for '{}' will be skipped (current target: '{}')",
                                tag, self.target_config.name
                            ),
                            _span,
                        );
                    }
                }
            }
            Stmt::Match { expr, arms } => {
                let scrutinee_ty = self.check_expr(&expr.node, expr.span);
                let mut has_wildcard = false;
                let mut has_true = false;
                let mut has_false = false;
                let mut wildcard_seen = false;

                for arm in arms {
                    if wildcard_seen {
                        self.error_with_help(
                            "unreachable pattern after wildcard '_'".to_string(),
                            arm.pattern.span,
                            "the wildcard `_` already matches all values; remove this arm or move it before `_`".to_string(),
                        );
                    }

                    match &arm.pattern.node {
                        MatchPattern::Literal(Literal::Integer(_)) => {
                            if scrutinee_ty != Ty::Field && scrutinee_ty != Ty::U32 {
                                self.error(
                                    format!(
                                        "integer pattern requires Field or U32 scrutinee, got {}",
                                        scrutinee_ty.display()
                                    ),
                                    arm.pattern.span,
                                );
                            }
                        }
                        MatchPattern::Literal(Literal::Bool(b)) => {
                            if scrutinee_ty != Ty::Bool {
                                self.error(
                                    format!(
                                        "boolean pattern requires Bool scrutinee, got {}",
                                        scrutinee_ty.display()
                                    ),
                                    arm.pattern.span,
                                );
                            }
                            if *b {
                                has_true = true;
                            } else {
                                has_false = true;
                            }
                        }
                        MatchPattern::Wildcard => {
                            has_wildcard = true;
                            wildcard_seen = true;
                        }
                        MatchPattern::Struct { name, fields } => {
                            // Look up the struct type
                            if let Some(sty) = self.structs.get(&name.node).cloned() {
                                // Verify scrutinee type matches the struct
                                if scrutinee_ty != Ty::Struct(sty.clone()) {
                                    self.error(
                                        format!(
                                            "struct pattern `{}` does not match scrutinee type `{}`",
                                            name.node,
                                            scrutinee_ty.display()
                                        ),
                                        arm.pattern.span,
                                    );
                                }
                                // Validate each field in the pattern
                                for spf in fields {
                                    if let Some((field_ty, _, _)) =
                                        sty.field_offset(&spf.field_name.node)
                                    {
                                        match &spf.pattern.node {
                                            FieldPattern::Literal(Literal::Integer(_)) => {
                                                if field_ty != Ty::Field && field_ty != Ty::U32 {
                                                    self.error(
                                                        format!(
                                                            "integer pattern on field `{}` requires Field or U32, got {}",
                                                            spf.field_name.node,
                                                            field_ty.display()
                                                        ),
                                                        spf.pattern.span,
                                                    );
                                                }
                                            }
                                            FieldPattern::Literal(Literal::Bool(_)) => {
                                                if field_ty != Ty::Bool {
                                                    self.error(
                                                        format!(
                                                            "boolean pattern on field `{}` requires Bool, got {}",
                                                            spf.field_name.node,
                                                            field_ty.display()
                                                        ),
                                                        spf.pattern.span,
                                                    );
                                                }
                                            }
                                            FieldPattern::Binding(_) | FieldPattern::Wildcard => {}
                                        }
                                    } else {
                                        self.error(
                                            format!(
                                                "struct `{}` has no field `{}`",
                                                name.node, spf.field_name.node
                                            ),
                                            spf.field_name.span,
                                        );
                                    }
                                }
                            } else {
                                self.error(
                                    format!("unknown struct type `{}`", name.node),
                                    name.span,
                                );
                            }
                        }
                    }

                    // For struct patterns, define bound variables in a scope wrapping the arm body
                    if let MatchPattern::Struct { name, fields } = &arm.pattern.node {
                        self.push_scope();
                        if let Some(sty) = self.structs.get(&name.node).cloned() {
                            for spf in fields {
                                if let FieldPattern::Binding(var_name) = &spf.pattern.node {
                                    if let Some((field_ty, _, _)) =
                                        sty.field_offset(&spf.field_name.node)
                                    {
                                        self.define_var(var_name, field_ty, false);
                                    }
                                }
                            }
                        }
                        self.check_block(&arm.body.node);
                        self.pop_scope();
                    } else {
                        self.check_block(&arm.body.node);
                    }
                }

                // Exhaustiveness: require wildcard unless Bool with both true+false,
                // or a struct pattern (structs have exactly one shape)
                let has_struct_pattern = arms
                    .iter()
                    .any(|a| matches!(a.pattern.node, MatchPattern::Struct { .. }));
                let exhaustive = has_wildcard
                    || (scrutinee_ty == Ty::Bool && has_true && has_false)
                    || has_struct_pattern;
                if !exhaustive {
                    self.error_with_help(
                        "non-exhaustive match: not all possible values are covered".to_string(),
                        expr.span,
                        "add a wildcard `_ => { ... }` arm to handle all remaining values"
                            .to_string(),
                    );
                }
            }
        }
    }
}
