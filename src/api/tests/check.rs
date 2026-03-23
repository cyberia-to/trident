// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use crate::*;

#[test]
fn test_check_valid_program() {
    let source = "program test\nfn main() {\n    pub_write(pub_read())\n}";
    assert!(check(source, "test.tri").is_ok());
}

#[test]
fn test_check_type_error() {
    let source = "program test\nfn main() {\n    let x: Bool = pub_read()\n}";
    assert!(check(source, "test.tri").is_err());
}

#[test]
fn test_check_silent_valid() {
    let source = "program test\nfn main() {\n    pub_write(pub_read())\n}";
    assert!(check_silent(source, "test.tri").is_ok());
}

#[test]
fn test_check_silent_error() {
    let source = "program test\nfn main() {\n    pub_write(undefined_var)\n}";
    assert!(check_silent(source, "test.tri").is_err());
}

#[test]
fn test_check_silent_parse_error() {
    let source = "program test\nfn main( {\n}";
    assert!(check_silent(source, "test.tri").is_err());
}

#[test]
fn test_array_of_structs_type_check() {
    // Arrays of structs should type-check correctly
    let source = r#"program test
struct Pt {
x: Field,
y: Field,
}

fn main() {
let a: Pt = Pt { x: 1, y: 2 }
let b: Pt = Pt { x: 3, y: 4 }
pub_write(a.x + b.y)
}
"#;
    assert!(check(source, "test.tri").is_ok());
}

#[test]
fn test_parse_source_silent_no_stderr() {
    // parse_source_silent should not render diagnostics
    let source = "program test\nfn main() {\n    pub_write(pub_read())\n}";
    let result = parse_source_silent(source, "test.tri");
    assert!(result.is_ok());
}

#[test]
fn test_parse_source_silent_returns_errors() {
    let source = "program test\nfn main( {\n}";
    let result = parse_source_silent(source, "test.tri");
    assert!(result.is_err());
}

#[test]
fn test_discover_tests_finds_test_fns() {
    let source = "program test\n#[test]\nfn check_math() {\n    assert(1 == 1)\n}\n#[test]\nfn check_logic() {\n    assert(true)\n}\nfn main() {}";
    let file = parse_source_silent(source, "test.tri").unwrap();
    let tests = discover_tests(&file);
    assert_eq!(tests.len(), 2);
    assert!(tests.contains(&"check_math".to_string()));
    assert!(tests.contains(&"check_logic".to_string()));
}

#[test]
fn test_discover_tests_empty_when_no_tests() {
    let source = "program test\nfn main() {\n    pub_write(pub_read())\n}";
    let file = parse_source_silent(source, "test.tri").unwrap();
    let tests = discover_tests(&file);
    assert!(tests.is_empty());
}

#[test]
fn test_test_fn_compiles_normally() {
    // #[test] functions should be accepted but skipped during normal emit
    let source = "program test\n#[test]\nfn check() {\n    assert(true)\n}\nfn main() {\n    pub_write(pub_read())\n}";
    let result = compile(source, "test.tri");
    assert!(
        result.is_ok(),
        "program with test fn should compile: {:?}",
        result.err()
    );
    let tasm = result.unwrap();
    // The test function should NOT appear in the emitted TASM
    assert!(
        !tasm.contains("__check:"),
        "test fn should not be emitted in normal build"
    );
    assert!(tasm.contains("__main:"), "main should be emitted");
}

#[test]
fn test_test_fn_type_check_valid() {
    let source = "program test\n#[test]\nfn check() {\n    assert(1 == 1)\n}\nfn main() {}";
    assert!(check(source, "test.tri").is_ok());
}

#[test]
fn test_test_fn_type_check_params_rejected() {
    let source =
        "program test\n#[test]\nfn bad(x: Field) {\n    assert(x == x)\n}\nfn main() {}";
    assert!(
        check(source, "test.tri").is_err(),
        "test fn with params should fail type check"
    );
}

#[test]
fn test_test_fn_type_check_return_rejected() {
    let source = "program test\n#[test]\nfn bad() -> Field {\n    42\n}\nfn main() {}";
    assert!(
        check(source, "test.tri").is_err(),
        "test fn with return should fail type check"
    );
}

