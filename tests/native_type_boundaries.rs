//! Type admission must prevent tree/scalar confusion before either backend.
use trident::{check_silent, CompileOptions, RAW_ARTIFACT_LIMITS};

#[test]
fn tuple_assignment_checks_each_component_and_requires_declared_targets() {
    for (body, expected) in [
        (
            "let mut a: Field = 0\nlet mut b: Field = 0\n(a,b) = (input,1)",
            "expected Field, got Noun",
        ),
        (
            "let mut a: Noun = input\nlet mut b: Field = 0\n(a,b) = (1,2)",
            "expected Noun, got Field",
        ),
        (
            "let mut a = [input]\nlet mut b: Field = 0\n(a,b) = ([1],2)",
            "tuple assignment to 'a'",
        ),
        (
            "let mut a: Field = 0\n(a,missing) = (1,2)",
            "undefined tuple assignment target 'missing'",
        ),
        (
            "let mut a: Field = 0\n(a,_) = (1,2)",
            "expected expression, found '_'",
        ),
        (
            "let mut a: Field = 0\nlet b: Field = 0\n(a,b) = (1,2)",
            "cannot assign to immutable variable 'b'",
        ),
    ] {
        let source = format!("program p\nfn main(input: Noun) -> Noun {{ {body}\ninput }}");
        let errors = format!("{:?}", check_silent(&source, "types.tri").unwrap_err());
        assert!(errors.contains(expected), "{errors}");
        assert!(trident::compile_raw_artifact(
            &source,
            "types.tri",
            &CompileOptions::default(),
            RAW_ARTIFACT_LIMITS
        )
        .is_err());
    }
    let valid = "program p\nfn main(input: Noun) -> Noun { let mut a: Noun = input\nlet mut b: Field = 0\n(a,b) = (nox_noun_pair(input,input),1)\na }";
    check_silent(valid, "types.tri").unwrap();
}

#[test]
fn loop_constant_classification_respects_parameter_let_and_index_shadowing() {
    let head = "program p\nconst n: Field = 9\n";
    for body in [
        "fn main(input: Field) -> Field { let n: Field = 2\nfor i in 0..n { }\ninput }",
        "fn main(n: Field) -> Field { for i in 0..n { }\nn }",
        "fn main(input: Field) -> Field { for n in 0..3 { for i in 0..n { } }\ninput }",
    ] {
        let errors = format!(
            "{:?}",
            check_silent(&format!("{head}{body}"), "types.tri").unwrap_err()
        );
        assert!(
            errors.contains("loop end must be a compile-time constant"),
            "{errors}"
        );
    }
    check_silent(
        &format!("{head}fn main(input: Field) -> Field {{ for i in 0..n {{ }}\ninput }}"),
        "types.tri",
    )
    .unwrap();
}
