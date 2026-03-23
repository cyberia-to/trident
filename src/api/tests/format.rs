// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use crate::*;

#[test]
fn test_format_source_valid() {
    let source = "program test\n\nfn main() {\n    pub_write(pub_read())\n}\n";
    let result = format_source(source, "test.tri");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), source);
}

#[test]
fn test_format_source_lex_error() {
    // Unterminated string or invalid character
    let source = "program test\n\nfn main() {\n    let x = @\n}\n";
    let result = format_source(source, "test.tri");
    assert!(result.is_err());
}

#[test]
fn test_generic_fn_format_roundtrip() {
    let source = "program test\n\nfn first<N>(arr: [Field; N]) -> Field {\n    arr[0]\n}\n\nfn main() {\n    let a: [Field; 3] = [1, 2, 3]\n    let s: Field = first<3>(a)\n    pub_write(s)\n}\n";
    let formatted = format_source(source, "test.tri").expect("should format");
    assert!(
        formatted.contains("<N>"),
        "formatted output should preserve <N>"
    );
    assert!(
        formatted.contains("first<3>"),
        "formatted output should preserve first<3>"
    );
}

#[test]
fn test_cfg_format_roundtrip() {
    let source = "program test\n\n#[cfg(debug)]\nfn check() {}\n\n#[cfg(release)]\nconst X: Field = 0\n\nfn main() {}\n";
    let formatted = format_source(source, "test.tri").expect("should format");
    assert!(
        formatted.contains("#[cfg(debug)]"),
        "should preserve cfg(debug)"
    );
    assert!(
        formatted.contains("#[cfg(release)]"),
        "should preserve cfg(release)"
    );
}

#[test]
fn test_match_format_roundtrip() {
    let source = "program test\n\nfn main() {\n    let x: Field = pub_read()\n    match x {\n        0 => {\n            pub_write(0)\n        }\n        1 => {\n            pub_write(1)\n        }\n        _ => {\n            pub_write(2)\n        }\n    }\n}\n";
    let formatted = format_source(source, "test.tri").unwrap();
    let formatted2 = format_source(&formatted, "test.tri").unwrap();
    assert_eq!(
        formatted, formatted2,
        "match formatting should be idempotent"
    );
}

#[test]
fn test_match_struct_pattern_format_roundtrip() {
    let source = "program test\n\nstruct Point {\n    x: Field,\n    y: Field,\n}\n\nfn main() {\n    let p = Point { x: 1, y: 2 }\n    match p {\n        Point { x, y } => {\n            pub_write(x)\n        }\n    }\n}\n";
    let formatted = format_source(source, "test.tri").unwrap();
    let formatted2 = format_source(&formatted, "test.tri").unwrap();
    assert_eq!(
        formatted, formatted2,
        "struct pattern formatting should be idempotent"
    );
}

#[test]
fn test_test_fn_format_roundtrip() {
    let source = "program test\n\n#[test]\nfn check_math() {\n    assert(1 == 1)\n}\n\nfn main() {\n    pub_write(pub_read())\n}\n";
    let formatted = format_source(source, "test.tri").unwrap();
    assert!(
        formatted.contains("#[test]"),
        "should preserve #[test] attribute"
    );
    assert!(
        formatted.contains("fn check_math()"),
        "should preserve function"
    );
    let formatted2 = format_source(&formatted, "test.tri").unwrap();
    assert_eq!(
        formatted, formatted2,
        "#[test] formatting should be idempotent"
    );
}

#[test]
fn test_test_fn_with_cfg_format_roundtrip() {
    let source = "program test\n\n#[cfg(debug)]\n#[test]\nfn debug_check() {\n    assert(true)\n}\n\nfn main() {}\n";
    let formatted = format_source(source, "test.tri").unwrap();
    assert!(formatted.contains("#[cfg(debug)]"), "should preserve cfg");
    assert!(formatted.contains("#[test]"), "should preserve test");
    let formatted2 = format_source(&formatted, "test.tri").unwrap();
    assert_eq!(
        formatted, formatted2,
        "cfg+test formatting should be idempotent"
    );
}

#[test]
fn test_pure_fn_format_roundtrip() {
    let source = "program test\n\n#[pure]\nfn add(a: Field, b: Field) -> Field {\n    a + b\n}\n\nfn main() {\n}\n";
    let formatted = format_source(source, "test.tri").unwrap();
    let formatted2 = format_source(&formatted, "test.tri").unwrap();
    assert_eq!(
        formatted, formatted2,
        "pure fn formatting should be idempotent"
    );
    assert!(
        formatted.contains("#[pure]"),
        "formatted output should contain #[pure]"
    );
}

