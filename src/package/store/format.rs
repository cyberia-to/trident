// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
pub(super) use crate::ast::display::format_ast_type as format_type;
use crate::ast::{self, Expr, Stmt};

// ─── Function Source Formatter ─────────────────────────────────────
//
// Reconstructs source from AST fields. This is a simple formatter
// for storage in the codebase; it does not need to handle comments
// or preserve formatting (format.rs does that for the full file).

pub(super) fn format_fn_source(func: &ast::FnDef) -> String {
    let mut out = String::new();

    if func.is_pub {
        out.push_str("pub ");
    }
    out.push_str("fn ");
    out.push_str(&func.name.node);

    // Type params.
    if !func.type_params.is_empty() {
        out.push('<');
        for (i, tp) in func.type_params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&tp.node);
        }
        out.push('>');
    }

    // Parameters.
    out.push('(');
    for (i, param) in func.params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&param.name.node);
        out.push_str(": ");
        out.push_str(&format_type(&param.ty.node));
    }
    out.push(')');

    // Return type.
    if let Some(ref ret) = func.return_ty {
        out.push_str(" -> ");
        out.push_str(&format_type(&ret.node));
    }

    // Body.
    match &func.body {
        Some(body) => {
            out.push_str(" {\n");
            format_block(&body.node, &mut out, 1);
            out.push('}');
        }
        None => {
            // Intrinsic/extern: no body.
        }
    }

    out
}

fn format_block(block: &ast::Block, out: &mut String, indent: usize) {
    let pad = "    ".repeat(indent);
    for stmt in &block.stmts {
        format_stmt(&stmt.node, out, &pad, indent);
    }
    if let Some(ref tail) = block.tail_expr {
        out.push_str(&pad);
        out.push_str(&format_expr(&tail.node));
        out.push('\n');
    }
}

fn format_stmt(stmt: &Stmt, out: &mut String, pad: &str, indent: usize) {
    match stmt {
        Stmt::Let {
            mutable,
            pattern,
            ty,
            init,
        } => {
            out.push_str(pad);
            out.push_str("let ");
            if *mutable {
                out.push_str("mut ");
            }
            match pattern {
                ast::Pattern::Name(name) => out.push_str(&name.node),
                ast::Pattern::Tuple(names) => {
                    out.push('(');
                    for (i, n) in names.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        out.push_str(&n.node);
                    }
                    out.push(')');
                }
            }
            if let Some(t) = ty {
                out.push_str(": ");
                out.push_str(&format_type(&t.node));
            }
            out.push_str(" = ");
            out.push_str(&format_expr(&init.node));
            out.push('\n');
        }
        Stmt::Assign { place, value } => {
            out.push_str(pad);
            out.push_str(&format_place(&place.node));
            out.push_str(" = ");
            out.push_str(&format_expr(&value.node));
            out.push('\n');
        }
        Stmt::TupleAssign { names, value } => {
            out.push_str(pad);
            out.push('(');
            for (i, n) in names.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&n.node);
            }
            out.push_str(") = ");
            out.push_str(&format_expr(&value.node));
            out.push('\n');
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            out.push_str(pad);
            out.push_str("if ");
            out.push_str(&format_expr(&cond.node));
            out.push_str(" {\n");
            format_block(&then_block.node, out, indent + 1);
            if let Some(else_blk) = else_block {
                out.push_str(pad);
                out.push_str("} else {\n");
                format_block(&else_blk.node, out, indent + 1);
            }
            out.push_str(pad);
            out.push_str("}\n");
        }
        Stmt::For {
            var,
            start,
            end,
            bound,
            body,
        } => {
            out.push_str(pad);
            out.push_str("for ");
            out.push_str(&var.node);
            out.push_str(" in ");
            out.push_str(&format_expr(&start.node));
            out.push_str("..");
            out.push_str(&format_expr(&end.node));
            if let Some(b) = bound {
                out.push_str(" bounded ");
                out.push_str(&b.to_string());
            }
            out.push_str(" {\n");
            format_block(&body.node, out, indent + 1);
            out.push_str(pad);
            out.push_str("}\n");
        }
        Stmt::Expr(expr) => {
            out.push_str(pad);
            out.push_str(&format_expr(&expr.node));
            out.push('\n');
        }
        Stmt::Return(val) => {
            out.push_str(pad);
            out.push_str("return");
            if let Some(v) = val {
                out.push(' ');
                out.push_str(&format_expr(&v.node));
            }
            out.push('\n');
        }
        Stmt::Reveal { event_name, fields } | Stmt::Seal { event_name, fields } => {
            let kw = if matches!(stmt, Stmt::Reveal { .. }) {
                "reveal"
            } else {
                "seal"
            };
            out.push_str(pad);
            out.push_str(kw);
            out.push(' ');
            out.push_str(&event_name.node);
            out.push_str(" { ");
            for (i, (name, val)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&name.node);
                out.push_str(": ");
                out.push_str(&format_expr(&val.node));
            }
            out.push_str(" }\n");
        }
        Stmt::Asm {
            body,
            effect,
            target,
        } => {
            out.push_str(pad);
            out.push_str("asm");
            match (target.as_deref(), *effect != 0) {
                (Some(tag), true) => {
                    if *effect > 0 {
                        out.push_str(&format!("({}, +{})", tag, effect));
                    } else {
                        out.push_str(&format!("({}, {})", tag, effect));
                    }
                }
                (Some(tag), false) => {
                    out.push_str(&format!("({})", tag));
                }
                (None, true) => {
                    if *effect > 0 {
                        out.push_str(&format!("(+{})", effect));
                    } else {
                        out.push_str(&format!("({})", effect));
                    }
                }
                (None, false) => {}
            }
            out.push_str(" {\n");
            let inner_pad = "    ".repeat(indent + 1);
            for line in body.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    out.push('\n');
                } else {
                    out.push_str(&inner_pad);
                    out.push_str(trimmed);
                    out.push('\n');
                }
            }
            out.push_str(pad);
            out.push_str("}\n");
        }
        Stmt::Match { expr, arms } => {
            out.push_str(pad);
            out.push_str("match ");
            out.push_str(&format_expr(&expr.node));
            out.push_str(" {\n");
            let arm_pad = "    ".repeat(indent + 1);
            for arm in arms {
                out.push_str(&arm_pad);
                match &arm.pattern.node {
                    ast::MatchPattern::Literal(ast::Literal::Integer(n)) => {
                        out.push_str(&n.to_string());
                    }
                    ast::MatchPattern::Literal(ast::Literal::Bool(b)) => {
                        out.push_str(if *b { "true" } else { "false" });
                    }
                    ast::MatchPattern::Wildcard => {
                        out.push('_');
                    }
                    ast::MatchPattern::Struct { name, fields } => {
                        out.push_str(&name.node);
                        out.push_str(" { ");
                        for (i, spf) in fields.iter().enumerate() {
                            if i > 0 {
                                out.push_str(", ");
                            }
                            out.push_str(&spf.field_name.node);
                            match &spf.pattern.node {
                                ast::FieldPattern::Binding(v) if v == &spf.field_name.node => {}
                                ast::FieldPattern::Binding(v) => {
                                    out.push_str(": ");
                                    out.push_str(v);
                                }
                                ast::FieldPattern::Literal(ast::Literal::Integer(n)) => {
                                    out.push_str(": ");
                                    out.push_str(&n.to_string());
                                }
                                ast::FieldPattern::Literal(ast::Literal::Bool(b)) => {
                                    out.push_str(": ");
                                    out.push_str(if *b { "true" } else { "false" });
                                }
                                ast::FieldPattern::Wildcard => {
                                    out.push_str(": _");
                                }
                            }
                        }
                        out.push_str(" }");
                    }
                }
                out.push_str(" => {\n");
                format_block(&arm.body.node, out, indent + 2);
                out.push_str(&arm_pad);
                out.push_str("}\n");
            }
            out.push_str(pad);
            out.push_str("}\n");
        }
    }
}

fn format_expr(expr: &Expr) -> String {
    match expr {
        Expr::Literal(ast::Literal::Integer(n)) => n.to_string(),
        Expr::Literal(ast::Literal::Bool(b)) => b.to_string(),
        Expr::Var(name) => name.clone(),
        Expr::BinOp { op, lhs, rhs } => {
            let l = format_expr_prec(&lhs.node, op, true);
            let r = format_expr_prec(&rhs.node, op, false);
            format!("{} {} {}", l, op.as_str(), r)
        }
        Expr::Call {
            path,
            generic_args,
            args,
        } => {
            let args_str: Vec<String> = args.iter().map(|a| format_expr(&a.node)).collect();
            if generic_args.is_empty() {
                format!("{}({})", path.node.as_dotted(), args_str.join(", "))
            } else {
                let ga: Vec<String> = generic_args.iter().map(|a| a.node.to_string()).collect();
                format!(
                    "{}<{}>({})",
                    path.node.as_dotted(),
                    ga.join(", "),
                    args_str.join(", ")
                )
            }
        }
        Expr::FieldAccess { expr, field } => {
            format!("{}.{}", format_expr(&expr.node), field.node)
        }
        Expr::Index { expr, index } => {
            format!("{}[{}]", format_expr(&expr.node), format_expr(&index.node))
        }
        Expr::StructInit { path, fields } => {
            let fields_str: Vec<String> = fields
                .iter()
                .map(|(name, val)| format!("{}: {}", name.node, format_expr(&val.node)))
                .collect();
            format!("{} {{ {} }}", path.node.as_dotted(), fields_str.join(", "))
        }
        Expr::ArrayInit(elems) => {
            let inner: Vec<String> = elems.iter().map(|e| format_expr(&e.node)).collect();
            format!("[{}]", inner.join(", "))
        }
        Expr::Tuple(elems) => {
            let inner: Vec<String> = elems.iter().map(|e| format_expr(&e.node)).collect();
            format!("({})", inner.join(", "))
        }
    }
}

fn format_expr_prec(expr: &Expr, parent_op: &ast::BinOp, _is_left: bool) -> String {
    if let Expr::BinOp { op, .. } = expr {
        if op_precedence(op) < op_precedence(parent_op) {
            return format!("({})", format_expr(expr));
        }
    }
    format_expr(expr)
}

fn op_precedence(op: &ast::BinOp) -> u8 {
    match op {
        ast::BinOp::Eq => 2,
        ast::BinOp::Lt => 4,
        ast::BinOp::Add => 6,
        ast::BinOp::Mul | ast::BinOp::XFieldMul => 8,
        ast::BinOp::BitAnd | ast::BinOp::BitXor => 10,
        ast::BinOp::DivMod => 12,
    }
}

fn format_place(place: &ast::Place) -> String {
    match place {
        ast::Place::Var(name) => name.clone(),
        ast::Place::FieldAccess(base, field) => {
            format!("{}.{}", format_place(&base.node), field.node)
        }
        ast::Place::Index(base, idx) => {
            format!("{}[{}]", format_place(&base.node), format_expr(&idx.node))
        }
    }
}
