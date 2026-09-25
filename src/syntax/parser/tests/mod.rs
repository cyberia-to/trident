// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
mod advanced;
mod basics;
mod block_boundary;
mod module_paths;

use crate::ast::File;
use crate::lexer::Lexer;
use crate::syntax::parser::Parser;

pub(super) fn parse(source: &str) -> File {
    let (tokens, _comments, lex_diags) = Lexer::new(source, 0).tokenize();
    assert!(lex_diags.is_empty(), "lex errors: {:?}", lex_diags);
    Parser::new(tokens).parse_file().unwrap()
}
