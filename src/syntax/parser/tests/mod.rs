// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
mod advanced;
mod basics;

use crate::syntax::parser::Parser;
use crate::ast::File;
use crate::lexer::Lexer;

pub(super) fn parse(source: &str) -> File {
    let (tokens, _comments, lex_diags) = Lexer::new(source, 0).tokenize();
    assert!(lex_diags.is_empty(), "lex errors: {:?}", lex_diags);
    Parser::new(tokens).parse_file().unwrap()
}
