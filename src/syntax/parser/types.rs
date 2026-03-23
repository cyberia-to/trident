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
    pub(super) fn parse_type(&mut self) -> Spanned<Type> {
        let start = self.current_span();
        let ty = match self.peek() {
            Lexeme::FieldTy => {
                self.advance();
                Type::Field
            }
            Lexeme::XFieldTy => {
                self.advance();
                Type::XField
            }
            Lexeme::BoolTy => {
                self.advance();
                Type::Bool
            }
            Lexeme::U32Ty => {
                self.advance();
                Type::U32
            }
            Lexeme::DigestTy => {
                self.advance();
                Type::Digest
            }
            Lexeme::LBracket => {
                self.advance();
                let inner = self.parse_type();
                self.expect(&Lexeme::Semicolon);
                let size = self.parse_array_size_expr();
                self.expect(&Lexeme::RBracket);
                Type::Array(Box::new(inner.node), size)
            }
            Lexeme::LParen => {
                self.advance();
                let mut types = vec![self.parse_type().node];
                while self.eat(&Lexeme::Comma) {
                    types.push(self.parse_type().node);
                }
                self.expect(&Lexeme::RParen);
                Type::Tuple(types)
            }
            Lexeme::Ident(_) => {
                let path = self.parse_module_path();
                Type::Named(path)
            }
            _ => {
                self.error_with_help(
                    "expected type",
                    "valid types are: Field, XField, Bool, U32, Digest, [T; N], (T, U), or a struct name",
                );
                Type::Field // fallback
            }
        };
        let span = start.merge(self.prev_span());
        Spanned::new(ty, span)
    }

    // --- Array size expression parsing (compile-time arithmetic) ---

    /// Parse a compile-time size expression: `N`, `3`, `M + N`, `N * 2`, `M + N * 2`.
    /// Precedence: `*` binds tighter than `+`.
    pub(super) fn parse_array_size_expr(&mut self) -> ArraySize {
        let mut left = self.parse_array_size_mul();
        while self.at(&Lexeme::Plus) {
            self.advance();
            let right = self.parse_array_size_mul();
            left = ArraySize::Add(Box::new(left), Box::new(right));
        }
        left
    }

    fn parse_array_size_mul(&mut self) -> ArraySize {
        let mut left = self.parse_array_size_atom();
        while self.at(&Lexeme::Star) {
            self.advance();
            let right = self.parse_array_size_atom();
            left = ArraySize::Mul(Box::new(left), Box::new(right));
        }
        left
    }

    fn parse_array_size_atom(&mut self) -> ArraySize {
        if let Lexeme::Integer(n) = self.peek() {
            let n = *n;
            self.advance();
            ArraySize::Literal(n)
        } else if let Lexeme::Ident(_) = self.peek() {
            let ident = self.expect_ident();
            ArraySize::Param(ident.node)
        } else if self.at(&Lexeme::LParen) {
            self.advance();
            let inner = self.parse_array_size_expr();
            self.expect(&Lexeme::RParen);
            inner
        } else {
            self.error_with_help(
                "expected array size (integer literal or size parameter name)",
                "array sizes are written as `N`, `3`, `M + N`, or `N * 2`",
            );
            ArraySize::Literal(0)
        }
    }
}
