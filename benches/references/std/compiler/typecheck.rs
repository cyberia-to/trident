// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use trident::api::CompileOptions;
use trident::lexer::Lexer;

/// Test source: a small Trident program with types to verify.
/// Exercises: let bindings, function calls, arithmetic, return types,
/// struct definitions, field access.
const TEST_SOURCE: &str = r#"program test

use vm.io.io

struct Point {
    x: Field,
    y: Field,
}

fn add(a: Field, b: Field) -> Field {
    let result: Field = a + b
    return result
}

fn make_point(x: Field, y: Field) -> Point {
    return Point { x: x, y: y }
}

fn main() {
    let x: Field = pub_read()
    let y: Field = 42
    let z: Field = add(x, y)
    let p: Point = make_point(x, y)
    pub_write(z)
    pub_write(p.x)
}
"#;

fn main() {
    let source = TEST_SOURCE;

    // Lex to get token count
    let (tokens, _comments, diags) = Lexer::new(source, 0).tokenize();
    assert!(diags.is_empty(), "lex errors: {:?}", diags);
    let tok_count = tokens.len();

    // Verify Rust compiler accepts this source (type checks clean)
    let options = CompileOptions::default();
    let tir_ops = trident::build_tir(source, "test.tri", &options)
        .expect("Rust compiler should compile test source cleanly");

    eprintln!("=== Typecheck Reference ===");
    eprintln!("Source: {} bytes", source.len());
    eprintln!("Tokens: {}", tok_count);
    eprintln!("Optimized TIR ops: {} (Rust compiler)", tir_ops.len());
    eprintln!("Expected type errors: 0");
    eprintln!();

    // Memory layout — must not overlap
    let src_base: u64 = 1000;
    let src_len = source.len() as u64;
    let tok_base: u64 = 2000;
    let lex_err_base: u64 = 3000;
    let lex_state_base: u64 = 3500;
    let ast_base: u64 = 5000;
    let parse_err_base: u64 = 8000;
    let parse_state_base: u64 = 9000;
    let parse_stack_base: u64 = 10000;
    let tc_state_base: u64 = 20000;
    let expected_err_count: u64 = 0;

    let mut vals: Vec<String> = Vec::new();

    // 12 parameters — match typecheck.tri read order
    vals.push(src_base.to_string());
    vals.push(src_len.to_string());
    vals.push(tok_base.to_string());
    vals.push(lex_err_base.to_string());
    vals.push(lex_state_base.to_string());
    vals.push(ast_base.to_string());
    vals.push(parse_err_base.to_string());
    vals.push(parse_state_base.to_string());
    vals.push(parse_stack_base.to_string());
    vals.push(tc_state_base.to_string());
    vals.push(tok_count.to_string());
    vals.push(expected_err_count.to_string());

    // Source bytes
    for &b in source.as_bytes() {
        vals.push((b as u64).to_string());
    }

    eprintln!("values: {}", vals.join(", "));
}
