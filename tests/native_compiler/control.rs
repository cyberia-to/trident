use super::support::{self, Result};

#[test]
fn native_branches_match_seed_types_scopes_and_return_continuations() {
    support::worker(|| {
        for (body, expected) in [
            ("if true {11} else {22}", 11),
            ("if false {11} else {22}", 22),
            ("let b:Bool=1+2*3==7 if b {11} else {22}", 11),
            ("let b:Bool=false==false if b {11} else {22}", 11),
            ("let mut b=true b=false if b {11} else {22}", 22),
            ("if 0 {11} else {22}", 11),
            ("if 1 {11} else {22}", 22),
            ("if 2 {11} else {22}", 22),
            ("if 18446744069414584321 {11} else {22}", 11),
            ("if true {7}", 7),
            ("if 0 {7}", 7),
            ("return 7", 7),
            ("if true {return 7} 9", 7),
            ("if false {return 7} 9", 9),
            ("if true {false} 7", 7),
            ("if true {if true {false} else {false}} 7", 7),
            ("1 2", 2),
            ("true 7", 7),
            ("if false {1} else if true {2} else {3}", 2),
            ("if false {1} else if false {2} else {3}", 3),
            ("if true {if false {1} else {2}} else {3}", 2),
            (
                "let mut x=0 if false{x=1}else if false{x=2}else{x=3} x=x+4 x",
                7,
            ),
            ("let mut x=3 if true {x=7 let x=9 x} else {let x=8 x} x", 7),
            ("let mut x=3 if false {x=7} else {x=8 let x=9 x} x", 8),
            ("let x=3 if true {let x=x+4 x} else {x}", 7),
            ("let mut x=1 if true {return x+6} else {x=9} x", 7),
            ("if true {let x=7 return x} else {return 9}", 7),
        ] {
            let source = support::source(body);
            assert_eq!(
                support::rust_value(std::str::from_utf8(&source).unwrap()),
                expected,
                "seed: {body}"
            );
            match support::compile(&source) {
                Result::Program {
                    value,
                    reductions,
                    nodes,
                    frames,
                    ..
                } => {
                    assert_eq!(value, expected, "guest: {body}");
                    eprintln!("{body}: reductions={reductions} nodes={nodes} frames={frames}");
                }
                other => panic!("{body}: {other:?}"),
            }
        }
    });
}

#[test]
fn native_control_rejects_type_scope_and_return_errors_in_every_arm() {
    support::worker(|| {
        for body in [
            "",
            "let x=1",
            "return",
            "true",
            "1==1",
            "let x:Bool=true x",
            "let x:Field=true 7",
            "let x:Bool=1 7",
            "let mut x=1 x=false x",
            "let mut b=true b=1 7",
            "true+false 7",
            "true*false 7",
            "1==true 7",
            "return true",
            "return 7 9",
            "return 7 let x=1",
            "if true {7} else {false}",
            "if false {false} else {7}",
            "if true {return false} 7",
            "if false {return false} 7",
            "if false {7}",
            "if 1 {7}",
            "if 2 {7}",
            "let b=true if b {7}",
            "if 1==1 {7}",
            "if true {let x=7} else {let x=9}",
            "if true {let x=7} x",
            "if true {let x=7} else {x} 9",
            "if true {let x=7} else if x==7 {1} else {2}",
            "if true {7} else {missing}",
            "let x=1 if false {x=2} 7",
        ] {
            let source = support::source(body);
            assert!(
                trident::compile(std::str::from_utf8(&source).unwrap(), "invalid.tri").is_err(),
                "seed accepted {body}"
            );
            support::error(&source, 5);
        }
        for body in [
            "if true 7",
            "if true {7",
            "if true {7} else",
            "if true {7} else 9",
            "else {7}",
        ] {
            support::error(&support::source(body), 2);
        }
        support::error(&support::source("let x=if true {7} else {9} x"), 6);
    });
}

#[test]
fn block_and_expression_caps_reject_before_growth_and_preserve_identity() {
    support::worker(|| {
        let source = support::source("if true {7}");
        let mut previous = None;
        for cap in [1, 2, 3, 4096] {
            let mut limits = support::CAPS;
            limits[2] = 1;
            limits[3] = cap;
            match support::compile_package(
                &[support::module(&source)],
                "sample",
                "main",
                support::options(),
                limits,
            ) {
                Result::Errors(errors) if cap == 1 => assert_eq!(errors[0].code, 7),
                Result::Program { bytes, value, .. } if cap >= 2 => {
                    assert_eq!(value, 7);
                    if let Some(ref expected) = previous {
                        assert_eq!(&bytes, expected);
                    }
                    previous = Some(bytes);
                }
                other => panic!("cap={cap}: {other:?}"),
            }
        }
    });
}

#[test]
fn block_table_cap_is_independent_of_expression_and_nesting_caps() {
    support::worker(|| {
        // Two expressions, one statement, three blocks: only the block append
        // rejects cap2, after both empty arms have already been accepted.
        let source = support::source("if true {} else {} 7");
        let mut previous = None;
        for cap in [2, 3, 4] {
            let mut limits = support::CAPS;
            limits[2] = 1;
            limits[3] = cap;
            match support::compile_package(
                &[support::module(&source)],
                "sample",
                "main",
                support::options(),
                limits,
            ) {
                Result::Errors(errors) if cap == 2 => {
                    assert_eq!(errors[0].code, 7);
                    assert_eq!(
                        &source[errors[0].start as usize..errors[0].end as usize],
                        b"}"
                    );
                    assert_eq!(errors[0].end as usize, source.len());
                }
                Result::Program { bytes, value, .. } if cap >= 3 => {
                    assert_eq!(value, 7);
                    if let Some(ref expected) = previous {
                        assert_eq!(&bytes, expected);
                    }
                    previous = Some(bytes);
                }
                other => panic!("cap={cap}: {other:?}"),
            }
        }
    });
}

#[test]
fn nested_control_crosses_parser_and_block_emitter_chunks() {
    support::worker(|| {
        for nesting in [7, 8, 9] {
            let body = format!("{}7{}", "if true {".repeat(nesting), "}".repeat(nesting));
            match support::compile(&support::source(&body)) {
                Result::Program {
                    value,
                    reductions,
                    nodes,
                    frames,
                    ..
                } => {
                    assert_eq!(value, 7);
                    eprintln!(
                        "nesting={nesting} reductions={reductions} nodes={nodes} frames={frames}"
                    );
                }
                other => panic!("nesting={nesting}: {other:?}"),
            }
        }
    });
}

#[test]
fn actual_parser_enter_charges_root_against_the_hard_nesting_ceiling() {
    support::worker(|| {
        let probe = trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_control_nesting.tri"),
            &trident::CompileOptions::default(),
            trident::NativeArtifactProfile::RawNoun,
            trident::NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap()
        .bytes;
        assert_eq!(support::run_input(&probe, 61), 62);
        assert_eq!(support::run_input(&probe, 62), 63);
        assert_eq!(support::run_input(&probe, 63), 7063);
    });
}
