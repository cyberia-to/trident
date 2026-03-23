// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::*;

fn lex(source: &str) -> Vec<Lexeme> {
    let (tokens, _comments, diags) = Lexer::new(source, 0).tokenize();
    assert!(diags.is_empty(), "unexpected errors: {:?}", diags);
    tokens.into_iter().map(|t| t.node).collect()
}

#[test]
fn test_keywords() {
    let tokens = lex("program fn let mut pub if else for in bounded return");
    assert_eq!(
        tokens,
        vec![
            Lexeme::Program,
            Lexeme::Fn,
            Lexeme::Let,
            Lexeme::Mut,
            Lexeme::Pub,
            Lexeme::If,
            Lexeme::Else,
            Lexeme::For,
            Lexeme::In,
            Lexeme::Bounded,
            Lexeme::Return,
            Lexeme::Eof,
        ]
    );
}

#[test]
fn test_types() {
    let tokens = lex("Field XField Bool U32 Digest");
    assert_eq!(
        tokens,
        vec![
            Lexeme::FieldTy,
            Lexeme::XFieldTy,
            Lexeme::BoolTy,
            Lexeme::U32Ty,
            Lexeme::DigestTy,
            Lexeme::Eof,
        ]
    );
}

#[test]
fn test_symbols() {
    let tokens = lex("( ) { } [ ] , : ; . .. -> = == + * *. < & ^ /% #");
    assert_eq!(
        tokens,
        vec![
            Lexeme::LParen,
            Lexeme::RParen,
            Lexeme::LBrace,
            Lexeme::RBrace,
            Lexeme::LBracket,
            Lexeme::RBracket,
            Lexeme::Comma,
            Lexeme::Colon,
            Lexeme::Semicolon,
            Lexeme::Dot,
            Lexeme::DotDot,
            Lexeme::Arrow,
            Lexeme::Eq,
            Lexeme::EqEq,
            Lexeme::Plus,
            Lexeme::Star,
            Lexeme::StarDot,
            Lexeme::Lt,
            Lexeme::Amp,
            Lexeme::Caret,
            Lexeme::SlashPercent,
            Lexeme::Hash,
            Lexeme::Eof,
        ]
    );
}

#[test]
fn test_integers() {
    let tokens = lex("0 1 42 18446744073709551615");
    assert_eq!(
        tokens,
        vec![
            Lexeme::Integer(0),
            Lexeme::Integer(1),
            Lexeme::Integer(42),
            Lexeme::Integer(u64::MAX),
            Lexeme::Eof,
        ]
    );
}

#[test]
fn test_identifiers() {
    let tokens = lex("foo bar_baz x1 _underscore");
    assert_eq!(
        tokens,
        vec![
            Lexeme::Ident("foo".into()),
            Lexeme::Ident("bar_baz".into()),
            Lexeme::Ident("x1".into()),
            Lexeme::Ident("_underscore".into()),
            Lexeme::Eof,
        ]
    );
}

#[test]
fn test_comments() {
    let tokens = lex("foo // this is a comment\nbar");
    assert_eq!(
        tokens,
        vec![
            Lexeme::Ident("foo".into()),
            Lexeme::Ident("bar".into()),
            Lexeme::Eof,
        ]
    );
}

#[test]
fn test_simple_program() {
    let tokens = lex("program test\n\nfn main() {\n    let a: Field = pub_read()\n}");
    assert_eq!(tokens[0], Lexeme::Program);
    assert_eq!(tokens[1], Lexeme::Ident("test".into()));
    assert_eq!(tokens[2], Lexeme::Fn);
    assert_eq!(tokens[3], Lexeme::Ident("main".into()));
}

#[test]
fn test_event_keywords() {
    let tokens = lex("event reveal seal");
    assert_eq!(
        tokens,
        vec![Lexeme::Event, Lexeme::Reveal, Lexeme::Seal, Lexeme::Eof,]
    );
}

#[test]
fn test_asm_block_basic() {
    let tokens = lex("asm { push 1\nadd }");
    assert_eq!(
        tokens,
        vec![
            Lexeme::AsmBlock {
                body: "push 1\nadd".to_string(),
                effect: 0,
                target: None,
            },
            Lexeme::Eof,
        ]
    );
}

#[test]
fn test_asm_block_positive_effect() {
    let tokens = lex("asm(+1) { push 42 }");
    assert_eq!(
        tokens,
        vec![
            Lexeme::AsmBlock {
                body: "push 42".to_string(),
                effect: 1,
                target: None,
            },
            Lexeme::Eof,
        ]
    );
}

#[test]
fn test_asm_block_negative_effect() {
    let tokens = lex("asm(-2) { pop 1\npop 1 }");
    assert_eq!(
        tokens,
        vec![
            Lexeme::AsmBlock {
                body: "pop 1\npop 1".to_string(),
                effect: -2,
                target: None,
            },
            Lexeme::Eof,
        ]
    );
}

#[test]
fn test_asm_block_with_negative_literal() {
    let tokens = lex("asm { push -1\nadd }");
    assert_eq!(
        tokens,
        vec![
            Lexeme::AsmBlock {
                body: "push -1\nadd".to_string(),
                effect: 0,
                target: None,
            },
            Lexeme::Eof,
        ]
    );
}

#[test]
fn test_asm_block_target_tag() {
    let tokens = lex("asm(triton) { push 1 }");
    assert_eq!(
        tokens,
        vec![
            Lexeme::AsmBlock {
                body: "push 1".to_string(),
                effect: 0,
                target: Some("triton".to_string()),
            },
            Lexeme::Eof,
        ]
    );
}

#[test]
fn test_asm_block_target_and_effect() {
    let tokens = lex("asm(triton, +2) { push 1\npush 2 }");
    assert_eq!(
        tokens,
        vec![
            Lexeme::AsmBlock {
                body: "push 1\npush 2".to_string(),
                effect: 2,
                target: Some("triton".to_string()),
            },
            Lexeme::Eof,
        ]
    );
}

#[test]
fn test_asm_block_in_function() {
    // fn main() { asm { ... } }
    // Tokens: Fn, Ident("main"), LParen, RParen, LBrace, AsmBlock, RBrace, Eof
    let tokens = lex("fn main() {\n    asm { dup 0\nadd }\n}");
    assert_eq!(tokens[0], Lexeme::Fn);
    assert!(matches!(tokens[5], Lexeme::AsmBlock { .. }));
    assert_eq!(tokens[6], Lexeme::RBrace);
}

#[test]
fn test_match_keyword() {
    let tokens = lex("match x { 0 => { } _ => { } }");
    assert_eq!(tokens[0], Lexeme::Match);
    assert_eq!(tokens[1], Lexeme::Ident("x".into()));
    assert_eq!(tokens[2], Lexeme::LBrace);
    assert_eq!(tokens[3], Lexeme::Integer(0));
    assert_eq!(tokens[4], Lexeme::FatArrow);
    assert_eq!(tokens[5], Lexeme::LBrace);
    assert_eq!(tokens[6], Lexeme::RBrace);
    assert_eq!(tokens[7], Lexeme::Underscore);
    assert_eq!(tokens[8], Lexeme::FatArrow);
}

#[test]
fn test_fat_arrow_vs_eq() {
    let tokens = lex("= => ==");
    assert_eq!(
        tokens,
        vec![Lexeme::Eq, Lexeme::FatArrow, Lexeme::EqEq, Lexeme::Eof]
    );
}

// --- Error path tests ---

fn lex_with_errors(source: &str) -> (Vec<Lexeme>, Vec<Diagnostic>) {
    let (tokens, _comments, diags) = Lexer::new(source, 0).tokenize();
    let lexemes = tokens.into_iter().map(|t| t.node).collect();
    (lexemes, diags)
}

#[test]
fn test_error_unexpected_character() {
    let (_tokens, diags) = lex_with_errors("@");
    assert!(!diags.is_empty(), "should produce an error for '@'");
    assert!(
        diags[0].message.contains("unexpected character '@'"),
        "error should name the character, got: {}",
        diags[0].message
    );
    assert!(
        diags[0].help.is_some(),
        "unexpected character error should have help text"
    );
}

#[test]
fn test_error_subtraction_operator() {
    let (_tokens, diags) = lex_with_errors("a - b");
    assert!(!diags.is_empty(), "should produce an error for '-'");
    assert!(
        diags[0].message.contains("no subtraction operator"),
        "should explain there is no subtraction, got: {}",
        diags[0].message
    );
    assert!(
        diags[0].help.as_deref().unwrap().contains("sub(a, b)"),
        "help should suggest sub() function"
    );
}

#[test]
fn test_error_division_operator() {
    let (_tokens, diags) = lex_with_errors("a / b");
    assert!(!diags.is_empty(), "should produce an error for '/'");
    assert!(
        diags[0].message.contains("no division operator"),
        "should explain there is no division, got: {}",
        diags[0].message
    );
    assert!(
        diags[0].help.as_deref().unwrap().contains("/%"),
        "help should suggest /% operator"
    );
}

#[test]
fn test_error_integer_too_large() {
    let (_tokens, diags) = lex_with_errors("99999999999999999999999");
    assert!(
        !diags.is_empty(),
        "should produce an error for huge integer"
    );
    assert!(
        diags[0].message.contains("too large"),
        "should say the integer is too large, got: {}",
        diags[0].message
    );
    assert!(
        diags[0].help.is_some(),
        "integer overflow error should have help text"
    );
}

#[test]
fn test_error_unterminated_asm_block() {
    let (_tokens, diags) = lex_with_errors("asm { push 1");
    assert!(
        !diags.is_empty(),
        "should produce an error for unterminated asm"
    );
    assert!(
        diags[0].message.contains("unterminated asm block"),
        "should report unterminated asm, got: {}",
        diags[0].message
    );
    assert!(
        diags[0].help.is_some(),
        "unterminated asm error should have help text"
    );
}
