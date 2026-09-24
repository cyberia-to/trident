use super::parse;
use crate::ast::*;
use crate::{lexer::Lexer, syntax::parser::Parser};

fn body(text: &str) -> Block {
    let file = parse(&format!("program boundary fn main() {{ {text} }}"));
    let Item::Fn(function) = file.items.into_iter().next().unwrap().node else {
        panic!("expected function")
    };
    function.body.unwrap().node
}

#[test]
fn uppercase_and_qualified_operands_leave_empty_and_name_bodies_intact() {
    for name in ["ZERO", "limits.ZERO"] {
        for content in ["", "value"] {
            let block = body(&format!(
                "if used == {name} {{ {content} }} else {{ other }}"
            ));
            let Stmt::If {
                cond,
                then_block,
                else_block,
            } = &block.stmts[0].node
            else {
                panic!("expected if")
            };
            let Expr::BinOp {
                op: BinOp::Eq, rhs, ..
            } = &cond.node
            else {
                panic!("expected equality")
            };
            assert!(matches!(&rhs.node, Expr::Var(value) if value == name));
            assert!(then_block.node.stmts.is_empty());
            if content.is_empty() {
                assert!(then_block.node.tail_expr.is_none());
            } else {
                assert!(
                    matches!(&then_block.node.tail_expr.as_ref().unwrap().node, Expr::Var(value) if value == content)
                );
            }
            assert!(
                matches!(&else_block.as_ref().unwrap().node.tail_expr.as_ref().unwrap().node, Expr::Var(value) if value == "other")
            );
        }
    }
}

#[test]
fn range_end_and_match_scrutinee_reserve_the_control_flow_brace() {
    let block = body("for i in 0..LIMIT + bounds.EXTRA {} match limits.ZERO {} ");
    let Stmt::For { end, body, .. } = &block.stmts[0].node else {
        panic!("expected for")
    };
    let Expr::BinOp { rhs, .. } = &end.node else {
        panic!("expected range addition")
    };
    assert!(matches!(&rhs.node, Expr::Var(value) if value == "bounds.EXTRA"));
    assert!(body.node.stmts.is_empty() && body.node.tail_expr.is_none());
    let Stmt::Match { expr, arms } = &block.stmts[1].node else {
        panic!("expected match")
    };
    assert!(matches!(&expr.node, Expr::Var(value) if value == "limits.ZERO"));
    assert!(arms.is_empty()); // Exhaustiveness belongs to type checking.
}

#[test]
fn delimited_subexpressions_keep_struct_literals_in_every_control_context() {
    for expression in [
        "(Point { x: 1 }).x",
        "value(Point { x: 1 })",
        "[Point { x: 1 }][0].x",
        "values[Point { x: 0 }.x]",
    ] {
        let block = body(&format!("if {expression} == 1 {{}} for i in 0..{expression} bounded 2 {{}} match {expression} {{ _ => {{}} }}"));
        assert_eq!(block.stmts.len(), 3, "{expression}");
        assert!(matches!(block.stmts[0].node, Stmt::If { .. }));
        assert!(matches!(block.stmts[1].node, Stmt::For { .. }));
        assert!(matches!(block.stmts[2].node, Stmt::Match { .. }));
    }
    let block = body("let empty = Empty {} let point = Point { x } if (Point { x: 1 }).x == 1 {}");
    for statement in &block.stmts[..2] {
        let Stmt::Let { init, .. } = &statement.node else {
            panic!("expected initializer")
        };
        assert!(matches!(init.node, Expr::StructInit { .. }));
    }
    let Stmt::If { cond, .. } = &block.stmts[2].node else {
        panic!("expected if")
    };
    let Expr::BinOp { lhs, .. } = &cond.node else {
        panic!("expected equality")
    };
    let Expr::FieldAccess { expr, .. } = &lhs.node else {
        panic!("expected projection")
    };
    assert!(matches!(expr.node, Expr::StructInit { .. }));
}

#[test]
fn undelimited_struct_literals_require_parentheses_before_blocks() {
    for statement in [
        "if Point { x: 1 }.x == 1 {}",
        "for i in 0..Point { x: 1 }.x bounded 2 {}",
        "match Point { x: 1 } { _ => {} }",
    ] {
        let source = format!("program boundary fn main() {{ {statement} }}");
        let (tokens, _, errors) = Lexer::new(&source, 0).tokenize();
        assert!(errors.is_empty());
        assert!(Parser::new(tokens).parse_file().is_err(), "{statement}");
    }
}
