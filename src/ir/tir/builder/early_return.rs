//! Eliminate source returns before structural lowering. Synthetic target blocks
//! are subroutines, so a machine `return` inside one would exit the wrong frame.
//! A typed result slot and flag keep all exits inside ordinary structured flow.
use crate::ast::*;
use crate::span::Spanned;

// `$` cannot occur in source identifiers; these names cannot capture user locals.
pub(super) const RESULT: &str = "$return_value";
pub(super) const FLAG: &str = "$return_done";
fn sp<T>(node: T) -> Spanned<T> {
    Spanned::dummy(node)
}
fn var(name: &str) -> Spanned<Expr> {
    sp(Expr::Var(name.into()))
}
fn assign(name: &str, value: Spanned<Expr>) -> Spanned<Stmt> {
    sp(Stmt::Assign {
        place: sp(Place::Var(name.into())),
        value,
    })
}
fn finish(value: Option<Spanned<Expr>>) -> Vec<Spanned<Stmt>> {
    let mut stmts = Vec::new();
    if let Some(value) = value {
        stmts.push(assign(RESULT, value));
    }
    stmts.push(assign(FLAG, sp(Expr::Literal(Literal::Bool(true)))));
    stmts
}
fn stmt_returns(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return(_) => true,
        Stmt::If {
            then_block,
            else_block,
            ..
        } => contains(&then_block.node) || else_block.as_ref().is_some_and(|b| contains(&b.node)),
        Stmt::For { body, .. } => contains(&body.node),
        Stmt::Match { arms, .. } => arms.iter().any(|a| contains(&a.body.node)),
        _ => false,
    }
}
pub(super) fn contains(block: &Block) -> bool {
    block.stmts.iter().any(|s| stmt_returns(&s.node))
}
fn guard(block: Block) -> Spanned<Stmt> {
    sp(Stmt::If {
        cond: sp(Expr::BinOp {
            op: BinOp::Eq,
            lhs: Box::new(var(FLAG)),
            rhs: Box::new(sp(Expr::Literal(Literal::Bool(false)))),
        }),
        then_block: sp(block),
        else_block: None,
    })
}
/// Guard a whole continuation, not individual statements: a declaration and all
/// its uses remain in the same lexical scope, and skipped branches add no slots.
fn sequence(stmts: &[Spanned<Stmt>], tail: &Option<Box<Spanned<Expr>>>, implicit: bool) -> Block {
    let mut result = Vec::new();
    for (index, original) in stmts.iter().enumerate() {
        if let Stmt::Return(value) = &original.node {
            result.extend(finish(value.clone()));
            // Keep unreachable call sites in traversal order: generic call
            // resolutions are currently indexed in original AST order.
            let rest = sequence(&stmts[index + 1..], tail, implicit);
            if !rest.stmts.is_empty() || rest.tail_expr.is_some() {
                result.push(guard(rest));
            }
            return Block {
                stmts: result,
                tail_expr: None,
            };
        }
        let final_result = implicit && tail.is_none() && index + 1 == stmts.len();
        let mut stmt = original.clone();
        match &mut stmt.node {
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                then_block.node = normalize(&then_block.node, final_result);
                if let Some(b) = else_block {
                    b.node = normalize(&b.node, final_result);
                }
            }
            Stmt::For { body, .. } => {
                let normalized = normalize(&body.node, false);
                body.node = if contains(&body.node) {
                    Block {
                        stmts: vec![guard(normalized)],
                        tail_expr: None,
                    }
                } else {
                    normalized
                };
            }
            Stmt::Match { arms, .. } => {
                for arm in arms {
                    arm.body.node = normalize(&arm.body.node, final_result);
                }
            }
            _ => {}
        }
        result.push(stmt);
        if stmt_returns(&original.node) {
            let rest = sequence(&stmts[index + 1..], tail, implicit);
            if !rest.stmts.is_empty() || rest.tail_expr.is_some() {
                result.push(guard(rest));
            }
            return Block {
                stmts: result,
                tail_expr: None,
            };
        }
    }
    if let Some(value) = tail {
        if implicit {
            result.extend(finish(Some((**value).clone())));
        } else {
            result.push(sp(Stmt::Expr((**value).clone())));
        }
    }
    Block {
        stmts: result,
        tail_expr: None,
    }
}
fn normalize(block: &Block, implicit: bool) -> Block {
    sequence(&block.stmts, &block.tail_expr, implicit)
}
pub(super) fn body(block: &Block, has_result: bool) -> Block {
    let mut block = normalize(block, has_result);
    if has_result {
        block.tail_expr = Some(Box::new(var(RESULT)));
    }
    block
}

/// A statement branch evaluates its tail for effects but contributes no value
/// to its enclosing block. Only a final conditional/match can forward a value;
/// loop iterations never do. Keep explicit returns unchanged for exit lowering.
pub(super) fn discard_statement_tails(block: &mut Block, value_context: bool) {
    let tail_position = block.tail_expr.is_none();
    let count = block.stmts.len();
    for (index, statement) in block.stmts.iter_mut().enumerate() {
        let forwards = value_context && tail_position && index + 1 == count;
        match &mut statement.node {
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                discard_statement_tails(&mut then_block.node, forwards);
                if let Some(other) = else_block {
                    discard_statement_tails(&mut other.node, forwards);
                }
            }
            Stmt::Match { arms, .. } => {
                for arm in arms {
                    discard_statement_tails(&mut arm.body.node, forwards);
                }
            }
            Stmt::For { body, .. } => discard_statement_tails(&mut body.node, false),
            _ => {}
        }
    }
    if !value_context {
        if let Some(tail) = block.tail_expr.take() {
            let span = tail.span;
            block.stmts.push(Spanned::new(Stmt::Expr(*tail), span));
        }
    }
}
