// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use trident::api::CompileOptions;

/// Test source: a small Trident program exercising the full pipeline.
/// Same source as used in the lex/parse/typecheck benches — simple enough
/// to fit in a proven program, complex enough to exercise all stages.
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
    let options = CompileOptions::default();

    // Run the full Rust compiler pipeline to get optimized TIR op count
    let tir_ops = trident::build_tir(source, "test.tri", &options)
        .expect("Rust compiler should compile test source cleanly");
    let tir_count = tir_ops.len() as u64;

    eprintln!("=== Pipeline Reference (full 5-stage) ===");
    eprintln!("Source: {} bytes", source.len());
    eprintln!("Optimized TIR ops: {}", tir_count);
    eprintln!();

    // Memory layout for the bench
    let state_base: u64 = 500;
    let src_base: u64 = 1000;
    let src_len = source.len() as u64;
    // scratch_base must be far enough from state_base to not overlap
    // Pipeline uses ~100K words of scratch
    let scratch_base: u64 = 200_000;
    let expected_err_count: u64 = 0;

    let mut vals: Vec<String> = Vec::new();

    // 6 parameters
    vals.push(state_base.to_string());
    vals.push(src_base.to_string());
    vals.push(src_len.to_string());
    vals.push(scratch_base.to_string());
    vals.push(tir_count.to_string());
    vals.push(expected_err_count.to_string());

    // Source bytes
    for &b in source.as_bytes() {
        vals.push((b as u64).to_string());
    }

    eprintln!("values: {}", vals.join(", "));
}
