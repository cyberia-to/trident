// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use std::time::Instant;

use trident::lexeme::Lexeme;
use trident::lexer::Lexer;

/// Map a Lexeme to the TK_* integer used by std/compiler/lexer.tri.
fn lexeme_to_tk(lexeme: &Lexeme) -> u64 {
    match lexeme {
        Lexeme::Program => 1,
        Lexeme::Module => 2,
        Lexeme::Use => 3,
        Lexeme::Fn => 4,
        Lexeme::Pub => 5,
        Lexeme::Sec => 6,
        Lexeme::Let => 7,
        Lexeme::Mut => 8,
        Lexeme::Const => 9,
        Lexeme::Struct => 10,
        Lexeme::If => 11,
        Lexeme::Else => 12,
        Lexeme::For => 13,
        Lexeme::In => 14,
        Lexeme::Bounded => 15,
        Lexeme::Return => 16,
        Lexeme::True => 17,
        Lexeme::False => 18,
        Lexeme::Event => 19,
        Lexeme::Reveal => 20,
        Lexeme::Seal => 21,
        Lexeme::Match => 22,
        Lexeme::FieldTy => 23,
        Lexeme::XFieldTy => 24,
        Lexeme::BoolTy => 25,
        Lexeme::U32Ty => 26,
        Lexeme::DigestTy => 27,
        Lexeme::LParen => 28,
        Lexeme::RParen => 29,
        Lexeme::LBrace => 30,
        Lexeme::RBrace => 31,
        Lexeme::LBracket => 32,
        Lexeme::RBracket => 33,
        Lexeme::Comma => 34,
        Lexeme::Colon => 35,
        Lexeme::Semicolon => 36,
        Lexeme::Dot => 37,
        Lexeme::DotDot => 38,
        Lexeme::Arrow => 39,
        Lexeme::Eq => 40,
        Lexeme::FatArrow => 41,
        Lexeme::EqEq => 42,
        Lexeme::Plus => 43,
        Lexeme::Star => 44,
        Lexeme::StarDot => 45,
        Lexeme::Lt => 46,
        Lexeme::Gt => 47,
        Lexeme::Amp => 48,
        Lexeme::Caret => 49,
        Lexeme::SlashPercent => 50,
        Lexeme::Hash => 51,
        Lexeme::Underscore => 52,
        Lexeme::Integer(_) => 53,
        Lexeme::Ident(_) => 54,
        Lexeme::AsmBlock { .. } => 55,
        Lexeme::Eof => 56,
    }
}

/// Test source: a small Trident program exercising keywords, types, symbols,
/// integers, identifiers, and comments.
const TEST_SOURCE: &str = r#"program test

use vm.io.io

fn add(a: Field, b: Field) -> Field {
    let result: Field = a + b
    return result
}

fn main() {
    let x: Field = pub_read()
    let y: Field = 42
    let z: Field = add(x, y)
    pub_write(z)
}
"#;

fn main() {
    let source = TEST_SOURCE;

    // Tokenize with the Rust lexer
    let (tokens, _comments, diags) = Lexer::new(source, 0).tokenize();
    assert!(diags.is_empty(), "unexpected errors: {:?}", diags);

    let tk_values: Vec<u64> = tokens.iter().map(|t| lexeme_to_tk(&t.node)).collect();
    let tok_count = tk_values.len();

    // Print token details to stderr for verification
    eprintln!("=== Lexer Reference ===");
    eprintln!("Source: {} bytes", source.len());
    eprintln!("Tokens: {}", tok_count);
    eprintln!();
    for (i, tok) in tokens.iter().enumerate() {
        eprintln!("  [{:3}] TK={:2}  {:?}", i, tk_values[i], tok.node);
    }
    eprintln!();

    // Generate .inputs format on stderr
    // Layout: src_base=1000, tok_base=2000, err_base=3000, state_base=4000
    let src_base: u64 = 1000;
    let src_len = source.len() as u64;
    let tok_base: u64 = 2000;
    let err_base: u64 = 3000;
    let state_base: u64 = 4000;
    let expected_tok_count = tok_count as u64;

    let mut vals: Vec<String> = Vec::new();
    // Parameters: src_base, src_len, tok_base, err_base, state_base, expected_tok_count
    vals.push(src_base.to_string());
    vals.push(src_len.to_string());
    vals.push(tok_base.to_string());
    vals.push(err_base.to_string());
    vals.push(state_base.to_string());
    vals.push(expected_tok_count.to_string());
    // Source bytes
    for &b in source.as_bytes() {
        vals.push((b as u64).to_string());
    }
    // Expected token kinds (for verification)
    for &tk in &tk_values {
        vals.push(tk.to_string());
    }

    eprintln!("values: {}", vals.join(", "));
    eprintln!();
    eprintln!("--- Expected token kinds ---");
    let tk_strs: Vec<String> = tk_values.iter().map(|v| v.to_string()).collect();
    eprintln!("  [{}]", tk_strs.join(", "));

    // Benchmark: tokenize many times
    for _ in 0..1000 {
        std::hint::black_box(Lexer::new(source, 0).tokenize());
    }

    let n = 100_000u128;
    let start = Instant::now();
    for _ in 0..n {
        std::hint::black_box(Lexer::new(std::hint::black_box(source), 0).tokenize());
    }
    println!("rust_ns: {}", start.elapsed().as_nanos() / n);
}
