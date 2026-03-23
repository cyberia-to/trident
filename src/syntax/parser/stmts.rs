// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use crate::ast::*;
use crate::lexeme::Lexeme;
use crate::span::Spanned;

use super::Parser;

impl Parser {
    pub(super) fn parse_block(&mut self) -> Spanned<Block> {
        if !self.enter_nesting() {
            let span = self.current_span();
            while !self.at(&Lexeme::Eof) {
                self.advance();
            }
            return Spanned::new(
                Block {
                    stmts: Vec::new(),
                    tail_expr: None,
                },
                span,
            );
        }

        let start = self.current_span();
        self.expect(&Lexeme::LBrace);

        let mut stmts = Vec::new();
        let mut tail_expr = None;

        while !self.at(&Lexeme::RBrace) && !self.at(&Lexeme::Eof) {
            if self.at(&Lexeme::Let) {
                stmts.push(self.parse_let_stmt());
            } else if self.at(&Lexeme::If) {
                stmts.push(self.parse_if_stmt());
            } else if self.at(&Lexeme::For) {
                stmts.push(self.parse_for_stmt());
            } else if self.at(&Lexeme::Return) {
                stmts.push(self.parse_return_stmt());
            } else if self.at(&Lexeme::Reveal) {
                stmts.push(self.parse_reveal_stmt());
            } else if self.at(&Lexeme::Seal) {
                stmts.push(self.parse_seal_stmt());
            } else if self.at(&Lexeme::Match) {
                stmts.push(self.parse_match_stmt());
            } else if matches!(self.peek(), Lexeme::AsmBlock { .. }) {
                let start = self.current_span();
                let tok = self.advance().clone();
                if let Lexeme::AsmBlock {
                    body,
                    effect,
                    target,
                } = &tok.node
                {
                    let span = start.merge(tok.span);
                    stmts.push(Spanned::new(
                        Stmt::Asm {
                            body: body.clone(),
                            effect: *effect,
                            target: target.clone(),
                        },
                        span,
                    ));
                }
            } else {
                // Parse as expression statement or tail expression
                let expr = self.parse_expr();

                if self.at(&Lexeme::RBrace) {
                    tail_expr = Some(Box::new(expr));
                } else if self.eat(&Lexeme::Eq) {
                    if let Expr::Tuple(elements) = &expr.node {
                        let names: Vec<Spanned<String>> = elements
                            .iter()
                            .map(|e| {
                                if let Expr::Var(name) = &e.node {
                                    Spanned::new(name.clone(), e.span)
                                } else {
                                    Spanned::new("_error_".to_string(), e.span)
                                }
                            })
                            .collect();
                        let value = self.parse_expr();
                        let span = expr.span.merge(value.span);
                        stmts.push(Spanned::new(Stmt::TupleAssign { names, value }, span));
                    } else {
                        let place = self.expr_to_place(&expr);
                        let value = self.parse_expr();
                        let span = expr.span.merge(value.span);
                        stmts.push(Spanned::new(Stmt::Assign { place, value }, span));
                    }
                } else {
                    let span = expr.span;
                    stmts.push(Spanned::new(Stmt::Expr(expr), span));
                }
            }
        }

        let end = self.current_span();
        self.expect(&Lexeme::RBrace);
        let span = start.merge(end);
        self.exit_nesting();
        Spanned::new(Block { stmts, tail_expr }, span)
    }

    fn parse_let_stmt(&mut self) -> Spanned<Stmt> {
        let start = self.current_span();
        self.expect(&Lexeme::Let);
        let mutable = self.eat(&Lexeme::Mut);

        let pattern = if self.eat(&Lexeme::LParen) {
            let mut names = Vec::new();
            while !self.at(&Lexeme::RParen) && !self.at(&Lexeme::Eof) {
                let name = if self.at(&Lexeme::Underscore) {
                    let span = self.current_span();
                    self.advance();
                    Spanned::new("_".to_string(), span)
                } else {
                    self.expect_ident()
                };
                names.push(name);
                if !self.eat(&Lexeme::Comma) {
                    break;
                }
            }
            self.expect(&Lexeme::RParen);
            Pattern::Tuple(names)
        } else if self.at(&Lexeme::Underscore) {
            let span = self.current_span();
            self.advance();
            Pattern::Name(Spanned::new("_".to_string(), span))
        } else {
            Pattern::Name(self.expect_ident())
        };

        let ty = if self.eat(&Lexeme::Colon) {
            Some(self.parse_type())
        } else {
            None
        };

        self.expect(&Lexeme::Eq);
        let init = self.parse_expr();
        let span = start.merge(init.span);
        Spanned::new(
            Stmt::Let {
                mutable,
                pattern,
                ty,
                init,
            },
            span,
        )
    }

    fn parse_if_stmt(&mut self) -> Spanned<Stmt> {
        let start = self.current_span();
        self.expect(&Lexeme::If);
        let cond = self.parse_expr();
        let then_block = self.parse_block();
        let else_block = if self.eat(&Lexeme::Else) {
            if self.at(&Lexeme::If) {
                let inner_if = self.parse_if_stmt();
                let span = inner_if.span;
                Some(Spanned::new(
                    Block {
                        stmts: vec![inner_if],
                        tail_expr: None,
                    },
                    span,
                ))
            } else {
                Some(self.parse_block())
            }
        } else {
            None
        };
        let span = start.merge(self.prev_span());
        Spanned::new(
            Stmt::If {
                cond,
                then_block,
                else_block,
            },
            span,
        )
    }

    fn parse_for_stmt(&mut self) -> Spanned<Stmt> {
        let start = self.current_span();
        self.expect(&Lexeme::For);

        let var = if self.at(&Lexeme::Underscore) {
            let span = self.current_span();
            self.advance();
            Spanned::new("_".to_string(), span)
        } else {
            self.expect_ident()
        };

        self.expect(&Lexeme::In);
        let range_start = self.parse_expr();
        self.expect(&Lexeme::DotDot);
        let range_end = self.parse_expr();

        let bound = if self.eat(&Lexeme::Bounded) {
            Some(self.expect_integer())
        } else {
            None
        };

        let body = self.parse_block();
        let span = start.merge(self.prev_span());
        Spanned::new(
            Stmt::For {
                var,
                start: range_start,
                end: range_end,
                bound,
                body,
            },
            span,
        )
    }

    fn parse_return_stmt(&mut self) -> Spanned<Stmt> {
        let start = self.current_span();
        self.expect(&Lexeme::Return);
        let value = if !self.at(&Lexeme::RBrace) && !self.at(&Lexeme::Eof) {
            Some(self.parse_expr())
        } else {
            None
        };
        let span = start.merge(self.prev_span());
        Spanned::new(Stmt::Return(value), span)
    }

    fn parse_reveal_stmt(&mut self) -> Spanned<Stmt> {
        let start = self.current_span();
        self.expect(&Lexeme::Reveal);
        let event_name = self.expect_ident();
        self.expect(&Lexeme::LBrace);
        let fields = self.parse_struct_init_fields();
        self.expect(&Lexeme::RBrace);
        let span = start.merge(self.prev_span());
        Spanned::new(Stmt::Reveal { event_name, fields }, span)
    }

    fn parse_seal_stmt(&mut self) -> Spanned<Stmt> {
        let start = self.current_span();
        self.expect(&Lexeme::Seal);
        let event_name = self.expect_ident();
        self.expect(&Lexeme::LBrace);
        let fields = self.parse_struct_init_fields();
        self.expect(&Lexeme::RBrace);
        let span = start.merge(self.prev_span());
        Spanned::new(Stmt::Seal { event_name, fields }, span)
    }

    fn parse_match_stmt(&mut self) -> Spanned<Stmt> {
        let start = self.current_span();
        self.expect(&Lexeme::Match);
        let expr = self.parse_expr();
        self.expect(&Lexeme::LBrace);

        let mut arms = Vec::new();
        while !self.at(&Lexeme::RBrace) && !self.at(&Lexeme::Eof) {
            let pat_start = self.current_span();
            let pattern = if self.at(&Lexeme::Underscore) {
                self.advance();
                MatchPattern::Wildcard
            } else if let Lexeme::Integer(n) = self.peek().clone() {
                self.advance();
                MatchPattern::Literal(Literal::Integer(n))
            } else if self.at(&Lexeme::True) {
                self.advance();
                MatchPattern::Literal(Literal::Bool(true))
            } else if self.at(&Lexeme::False) {
                self.advance();
                MatchPattern::Literal(Literal::Bool(false))
            } else if matches!(self.peek(), Lexeme::Ident(_))
                && matches!(self.tokens[self.pos + 1].node, Lexeme::LBrace)
            {
                self.parse_struct_match_pattern()
            } else {
                self.error_with_help(
                    "expected match pattern (integer, true, false, StructName { ... }, or _)",
                    "match arms use literal patterns like `0 =>`, `true =>`, struct patterns like `Point { x, y } =>`, or wildcard `_ =>`",
                );
                self.advance();
                MatchPattern::Wildcard
            };
            let pat_span = pat_start.merge(self.prev_span());

            self.expect(&Lexeme::FatArrow);
            let body = self.parse_block();

            arms.push(MatchArm {
                pattern: Spanned::new(pattern, pat_span),
                body,
            });

            // Optional comma between arms
            self.eat(&Lexeme::Comma);
        }

        self.expect(&Lexeme::RBrace);
        let span = start.merge(self.prev_span());
        Spanned::new(Stmt::Match { expr, arms }, span)
    }

    /// Parse a struct destructuring pattern: `Point { x, y: 0, z: _ }`.
    fn parse_struct_match_pattern(&mut self) -> MatchPattern {
        let name = self.expect_ident();
        self.expect(&Lexeme::LBrace);

        let mut fields = Vec::new();
        while !self.at(&Lexeme::RBrace) && !self.at(&Lexeme::Eof) {
            let field_name = self.expect_ident();

            let pattern = if self.eat(&Lexeme::Colon) {
                let pat_start = self.current_span();
                let pat = if self.at(&Lexeme::Underscore) {
                    self.advance();
                    FieldPattern::Wildcard
                } else if let Lexeme::Integer(n) = self.peek().clone() {
                    self.advance();
                    FieldPattern::Literal(Literal::Integer(n))
                } else if self.at(&Lexeme::True) {
                    self.advance();
                    FieldPattern::Literal(Literal::Bool(true))
                } else if self.at(&Lexeme::False) {
                    self.advance();
                    FieldPattern::Literal(Literal::Bool(false))
                } else if matches!(self.peek(), Lexeme::Ident(_)) {
                    let binding = self.expect_ident();
                    FieldPattern::Binding(binding.node)
                } else {
                    self.error_with_help(
                        "expected field pattern (identifier, literal, or _)",
                        "use `field: var` to bind, `field: 0` to match, or `field: _` to ignore",
                    );
                    self.advance();
                    FieldPattern::Wildcard
                };
                let pat_span = pat_start.merge(self.prev_span());
                Spanned::new(pat, pat_span)
            } else {
                let span = field_name.span;
                Spanned::new(FieldPattern::Binding(field_name.node.clone()), span)
            };

            fields.push(StructPatternField {
                field_name,
                pattern,
            });

            if !self.eat(&Lexeme::Comma) {
                break;
            }
        }

        self.expect(&Lexeme::RBrace);
        MatchPattern::Struct { name, fields }
    }
}
