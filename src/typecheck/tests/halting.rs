use super::{check, check_err, check_with_flags};

#[test]
fn direct_assertion_failure_ends_typed_returns_and_nested_branches() {
    for body in [
        "assert(false)",
        "return assert(false)",
        "if c { assert(false) } else { 7 }",
        "if c { if c {assert(false)} else {assert(false)} } else {7}",
        "match c {true=>{assert(false)} false=>{7}}",
        "for i in 0..2 {assert(false)}",
    ] {
        assert!(
            check(&format!(
                "program test\nfn choose(c:Bool)->Field {{ {body} }}\nfn main() {{}} "
            ))
            .is_ok(),
            "{body}"
        );
    }
    for body in ["assert(true)", "assert(c)", "for i in 0..0 {assert(false)}"] {
        assert!(
            !check_err(&format!(
                "program test\nfn choose(c:Bool)->Field {{ {body} }}\nfn main() {{}} "
            ))
            .is_empty(),
            "{body}"
        );
    }
}

#[test]
fn ordinary_and_generic_assert_functions_keep_their_return_semantics() {
    for function in [
        "fn assert(c:Bool)->Field {7}",
        "fn assert<N>(c:Bool)->Field {7}",
    ] {
        let call = if function.contains('<') {
            "assert<2>(false)"
        } else {
            "assert(false)"
        };
        assert!(check(&format!(
            "program test\n{function}\nfn main() {{ let value={call} pub_write(value) }}"
        ))
        .is_ok());
        assert!(check(&format!(
            "program test\n{function}\nfn main() {{ {call} pub_write(11) }}"
        ))
        .is_ok());
    }
}

#[test]
fn direct_intrinsic_identity_tracks_active_declarations() {
    let source="module std.failure\n#[cfg(debug)] #[intrinsic(assert)] pub fn stop(c:Bool)\n#[cfg(release)] pub fn stop(c:Bool)->Field {7}\n";
    let debug = check_with_flags(source, &["debug"]).unwrap();
    let release = check_with_flags(source, &["release"]).unwrap();
    assert_eq!(
        debug.direct_intrinsics.get("stop").map(String::as_str),
        Some("assert")
    );
    assert!(!release.direct_intrinsics.contains_key("stop"));
}

#[test]
fn unreachable_continuations_follow_direct_failure_paths() {
    for body in [
        "assert(false) pub_write(7)",
        "return assert(false) pub_write(7)",
    ] {
        let diagnostics = check_err(&format!("program test\nfn main(c:Bool) {{ {body} }}"));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("unreachable")),
            "{diagnostics:?}"
        );
    }
}

#[test]
fn imported_intrinsic_identity_is_distinct_from_wrapper_requirements() {
    use crate::{lexer::Lexer, parser::Parser, typecheck::TypeChecker};
    let module=check("module std.failure\n#[intrinsic(assert)] pub fn stop(c:Bool)\npub fn wrapper(c:Bool) {stop(c)}\n").unwrap();
    assert!(module.function_requirements["wrapper"].contains("assert"));
    assert!(!module.direct_intrinsics.contains_key("wrapper"));
    for (call, accepted) in [
        ("failure.stop(false)", true),
        ("std.failure.stop(false)", true),
        ("failure.wrapper(false)", false),
    ] {
        let source = format!("program test\nfn main()->Field {{ {call} }}");
        let (tokens, _, _) = Lexer::new(&source, 0).tokenize();
        let file = Parser::new(tokens).parse_file().unwrap();
        let mut checker = TypeChecker::with_target(crate::target::TerrainConfig::triton())
            .with_intrinsics(&crate::target::TerrainConfig::test_intrinsics());
        checker.import_module(&module);
        let result = checker.check_file(&file);
        assert_eq!(result.is_ok(), accepted, "{call}: {result:?}");
    }
}

#[test]
fn field_wrapping_cannot_prove_a_loop_produces_a_return() {
    for end in ["18446744069414584321", "18446744069414584322"] {
        let missing = format!("program test\nfn choose()->Field {{for i in 0..{end} {{assert(false)}}}}\nfn main() {{}}");
        assert!(!check_err(&missing).is_empty(), "{end}");
        let fallback = format!("program test\nfn choose()->Field {{for i in 0..{end} {{assert(false)}} 7}}\nfn main() {{}}");
        assert!(check(&fallback).is_ok(), "{end}");
    }
}

#[test]
fn a_local_root_shadows_imported_constants_in_return_coverage() {
    use crate::{lexer::Lexer, parser::Parser, typecheck::TypeChecker};
    let module = check("module constants\npub const FLAG:Field=1\npub const END:U32=1").unwrap();
    for body in [
        "if constants.FLAG {assert(false)}",
        "for i in 0..constants.END bounded 1 {assert(false)}",
    ] {
        let source=format!("program test\nstruct Dynamic {{FLAG:Field,END:U32}}\nfn choose(constants:Dynamic)->Field {{{body}}}\nfn main() {{}}");
        let (tokens, _, _) = Lexer::new(&source, 0).tokenize();
        let file = Parser::new(tokens).parse_file().unwrap();
        let mut checker = TypeChecker::with_target(crate::target::TerrainConfig::triton())
            .with_intrinsics(&crate::target::TerrainConfig::test_intrinsics());
        checker.import_module(&module);
        assert!(
            checker
                .check_file(&file)
                .unwrap_err()
                .iter()
                .any(|d| d.message.contains("return type mismatch")),
            "{body}"
        );
    }
}
