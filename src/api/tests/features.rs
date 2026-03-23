// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use crate::*;

#[test]
fn test_generic_fn_compile_explicit() {
    let source = r#"program test

fn first<N>(arr: [Field; N]) -> Field {
arr[0]
}

fn main() {
let a: [Field; 3] = [1, 2, 3]
let s: Field = first<3>(a)
pub_write(s)
}
"#;
    let result = compile(source, "test.tri");
    assert!(
        result.is_ok(),
        "generic fn should compile: {:?}",
        result.err()
    );
    let tasm = result.unwrap();
    assert!(
        tasm.contains("__first__N3:"),
        "should emit monomorphized label"
    );
}

#[test]
fn test_generic_fn_compile_inferred() {
    let source = r#"program test

fn first<N>(arr: [Field; N]) -> Field {
arr[0]
}

fn main() {
let a: [Field; 3] = [1, 2, 3]
let s: Field = first(a)
pub_write(s)
}
"#;
    let result = compile(source, "test.tri");
    assert!(
        result.is_ok(),
        "generic fn with inference should compile: {:?}",
        result.err()
    );
}

#[test]
fn test_generic_fn_type_error() {
    let source = r#"program test

fn first<N>(arr: [Field; N]) -> Field {
arr[0]
}

fn main() {
let a: [Field; 3] = [1, 2, 3]
let s: Field = first<5>(a)
}
"#;
    let result = compile(source, "test.tri");
    assert!(result.is_err(), "wrong size arg should fail compilation");
}

#[test]
fn test_generic_fn_multiple_instantiations_compile() {
    let source = r#"program test

fn first<N>(arr: [Field; N]) -> Field {
arr[0]
}

fn main() {
let a: [Field; 3] = [1, 2, 3]
let b: [Field; 5] = [1, 2, 3, 4, 5]
pub_write(first<3>(a) + first<5>(b))
}
"#;
    let result = compile(source, "test.tri");
    assert!(
        result.is_ok(),
        "multiple instantiations should compile: {:?}",
        result.err()
    );
    let tasm = result.unwrap();
    assert!(tasm.contains("__first__N3:"));
    assert!(tasm.contains("__first__N5:"));
}

#[test]
fn test_generic_fn_existing_code_unaffected() {
    // Non-generic code should still work exactly as before
    let source = r#"program test

fn add(a: Field, b: Field) -> Field {
a + b
}

fn main() {
let x: Field = pub_read()
let y: Field = pub_read()
pub_write(add(x, y))
}
"#;
    let result = compile(source, "test.tri");
    assert!(result.is_ok());
    let tasm = result.unwrap();
    assert!(tasm.contains("call __add"));
    assert!(tasm.contains("__add:"));
}

#[test]
fn test_generic_fn_check_only() {
    let source = r#"program test

fn sum<N>(arr: [Field; N]) -> Field {
arr[0]
}

fn main() {
let a: [Field; 3] = [1, 2, 3]
let s: Field = sum<3>(a)
pub_write(s)
}
"#;
    assert!(
        check(source, "test.tri").is_ok(),
        "type-check only should work"
    );
}

#[test]
fn test_const_generic_add_expression() {
    // Parameter type uses M + N size expression
    let source = "program test\nfn first_of<M, N>(a: [Field; M + N]) -> Field {\n    a[0]\n}\nfn main() {\n    let a: [Field; 5] = [1, 2, 3, 4, 5]\n    let r = first_of<3, 2>(a)\n    assert(r == 1)\n}";
    let result = compile(source, "test.tri");
    assert!(
        result.is_ok(),
        "const generic add should compile: {:?}",
        result.err()
    );
}

#[test]
fn test_const_generic_mul_expression() {
    // Parameter type uses N * 2 size expression
    let source = "program test\nfn sum_pairs<N>(a: [Field; N * 2]) -> Field {\n    a[0] + a[1]\n}\nfn main() {\n    let a: [Field; 4] = [1, 2, 3, 4]\n    let r = sum_pairs<2>(a)\n    assert(r == 3)\n}";
    let result = compile(source, "test.tri");
    assert!(
        result.is_ok(),
        "const generic mul should compile: {:?}",
        result.err()
    );
}

#[test]
fn test_cfg_debug_compiles() {
    let source = "program test\n#[cfg(debug)]\nfn check() {\n    assert(true)\n}\nfn main() {\n    check()\n}";
    let options = CompileOptions::for_target("debug");
    let result = compile_with_options(source, "test.tri", &options);
    assert!(result.is_ok(), "debug cfg should compile in debug mode");
    let tasm = result.unwrap();
    assert!(tasm.contains("__check:"), "check fn should be emitted");
}

#[test]
fn test_cfg_release_excludes_debug_fn() {
    let source = "program test\n#[cfg(debug)]\nfn check() {\n    assert(true)\n}\nfn main() {}";
    let options = CompileOptions::for_target("release");
    let result = compile_with_options(source, "test.tri", &options);
    assert!(result.is_ok(), "should compile without debug fn");
    let tasm = result.unwrap();
    assert!(
        !tasm.contains("__check:"),
        "check fn should NOT be emitted in release"
    );
}

#[test]
fn test_cfg_different_targets_different_output() {
    let source = "program test\n#[cfg(debug)]\nfn mode() -> Field { 0 }\n#[cfg(release)]\nfn mode() -> Field { 1 }\nfn main() {\n    let x: Field = mode()\n    pub_write(x)\n}";

    let debug_opts = CompileOptions::for_target("debug");
    let debug_tasm =
        compile_with_options(source, "test.tri", &debug_opts).expect("debug should compile");

    let release_opts = CompileOptions::for_target("release");
    let release_tasm = compile_with_options(source, "test.tri", &release_opts)
        .expect("release should compile");

    // Both should have __mode: but with different bodies
    assert!(debug_tasm.contains("__mode:"));
    assert!(release_tasm.contains("__mode:"));
    // Debug pushes 0, release pushes 1
    assert!(debug_tasm.contains("push 0"));
    assert!(release_tasm.contains("push 1"));
}

#[test]
fn test_cfg_const_excluded_in_release() {
    let source = "program test\n#[cfg(debug)]\nconst LEVEL: Field = 3\nfn main() {}";
    let options = CompileOptions::for_target("release");
    let result = compile_with_options(source, "test.tri", &options);
    assert!(
        result.is_ok(),
        "should compile even though const is excluded"
    );
}

#[test]
fn test_no_cfg_backward_compatible() {
    // All existing code should work unchanged (no cfg = always active)
    let source = "program test\nfn helper() -> Field { 42 }\nfn main() {\n    let x: Field = helper()\n    pub_write(x)\n}";
    let result = compile(source, "test.tri");
    assert!(result.is_ok(), "no-cfg code should compile as before");
}

#[test]
fn test_match_compiles() {
    let source = "program test\nfn main() {\n    let x: Field = pub_read()\n    match x {\n        0 => { pub_write(0) }\n        1 => { pub_write(1) }\n        _ => { pub_write(2) }\n    }\n}";
    let result = compile(source, "test.tri");
    assert!(result.is_ok(), "match should compile: {:?}", result.err());
    let tasm = result.unwrap();
    assert!(tasm.contains("eq"), "match should emit equality checks");
    assert!(tasm.contains("skiz"), "match should use skiz for branching");
}

#[test]
fn test_match_bool_compiles() {
    let source = "program test\nfn main() {\n    let b: Bool = pub_read() == pub_read()\n    match b {\n        true => { pub_write(1) }\n        false => { pub_write(0) }\n    }\n}";
    let result = compile(source, "test.tri");
    assert!(
        result.is_ok(),
        "bool match should compile: {:?}",
        result.err()
    );
}

#[test]
fn test_match_non_exhaustive_fails() {
    let source = "program test\nfn main() {\n    let x: Field = pub_read()\n    match x {\n        0 => { pub_write(0) }\n    }\n}";
    let result = compile(source, "test.tri");
    assert!(result.is_err(), "non-exhaustive match should fail");
}

#[test]
fn test_match_struct_pattern_compiles() {
    let source = "program test\nstruct Point { x: Field, y: Field }\nfn main() {\n    let p = Point { x: pub_read(), y: pub_read() }\n    match p {\n        Point { x, y } => {\n            pub_write(x)\n            pub_write(y)\n        }\n    }\n}";
    let result = compile(source, "test.tri");
    assert!(
        result.is_ok(),
        "struct pattern should compile: {:?}",
        result.err()
    );
    let tasm = result.unwrap();
    assert!(tasm.contains("read_io"), "should read inputs");
    assert!(tasm.contains("write_io"), "should write outputs");
}

#[test]
fn test_pure_fn_compiles() {
    let source = "program test\n#[pure]\nfn add(a: Field, b: Field) -> Field {\n    a + b\n}\nfn main() {\n    let x = add(1, 2)\n    assert(x == 3)\n}";
    let result = compile(source, "test.tri");
    assert!(result.is_ok(), "pure fn should compile: {:?}", result.err());
}

