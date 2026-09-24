use super::support::{self, Compilation};

fn caps() -> [u64; 11] {
    let mut caps = support::CAPS;
    caps[9] = 786432;
    caps
}

fn program(source: &str) -> Vec<u8> {
    match support::compile_only(source.as_bytes(), caps()) {
        Compilation::Program { bytes, .. } => bytes,
        other => panic!("{source}: {other:?}"),
    }
}

#[test]
fn typed_scalars_preserve_unsigned_values_masks_and_checked_conversions() {
    support::worker(|| {
        for (body, expected) in [
            ("as_field(as_u32(0))", 0),
            ("as_field(as_u32(4294967295))", 4294967295),
            ("as_field(as_u32(18446744069414584321))", 0),
            ("as_field(as_u32(18446744069414584322))", 1),
            ("let x:U32=as_u32(7) as_field(x)", 7),
            (
                "let mut x=as_u32(7) let y=x x=as_u32(9) as_field(y)*10+as_field(x)",
                79,
            ),
            ("let x=as_u32(7) let x=as_u32(as_field(x)+1) as_field(x)", 8),
            (
                "as_field(as_u32(4294967295)&as_u32(4278190080))",
                4278190080,
            ),
            ("as_field(as_u32(7)&as_u32(6)&as_u32(3))", 2),
            ("if as_u32(0)<as_u32(4294967295){7}else{9}", 7),
            ("if as_u32(4294967295)<as_u32(4294967295){7}else{9}", 9),
            ("if as_u32(2)<as_u32(3)&as_u32(1){7}else{9}", 9),
            ("if as_u32(2)<as_u32(3)==true{7}else{9}", 7),
            ("if as_u32(7)==as_u32(7){7}else{9}", 7),
            ("sub(2,3)", 18446744069414584320),
            ("sub(sub(20,3),sub(7,2))", 12),
            ("as_field(as_u32(sub(as_field(as_u32(9)),1)))", 8),
            ("as_field(as_u32(7,),)", 7),
        ] {
            let source = String::from_utf8(support::source(body)).unwrap();
            let bytes = program(&source);
            assert_eq!(support::run_artifact(&bytes), expected, "{source}");
            assert_eq!(support::rust_value(&source), expected, "seed: {source}");
        }
    });
}

#[test]
fn user_functions_override_builtins_without_local_name_capture() {
    support::worker(|| {
        for (body, expected) in [
            ("fn f(x:U32)->U32{x} fn main()->Field{as_field(f(as_u32(9)))}",9),
            ("fn f(x:Field,x:U32)->U32{x} fn main()->Field{as_field(f(7,as_u32(9)))}",9),
            ("fn main()->Field{as_u32(7)} fn as_u32(x:Field)->Field{x+2}",9),
            ("fn as_field(x:Bool)->Field{if x{7}else{9}} fn main()->Field{as_field(true)}",7),
            ("fn sub(x:Field,y:Field)->Field{x*10+y} fn main()->Field{sub(7,2)}",72),
            ("fn as_u32(x:Field)->Field{1} fn as_u32(x:Bool)->Field{if x{7}else{9}} fn main()->Field{as_u32(true)}",7),
            ("fn main()->Field{let as_u32=9 let as_field=8 as_field(as_u32(7))+as_field}",15),
            ("fn main()->Field{let sub=9 sub(7,2)+sub}",14),
        ] {
            let source = format!("program sample {body}");
            assert_eq!(support::run_artifact(&program(&source)), expected, "{source}");
            assert_eq!(support::rust_value(&source), expected, "seed: {source}");
        }
    });
}

#[test]
fn scalar_type_errors_are_rejected_before_any_program_is_published() {
    support::worker(|| {
        for body in [
            "let x:U32=7 as_field(x)",
            "as_field(7)",
            "as_u32(true)",
            "as_u32(as_u32(7))",
            "as_u32()",
            "as_u32(1,2)",
            "as_field()",
            "sub(1)",
            "sub(1,2,3)",
            "sub(as_u32(1),2)",
            "as_fiele(as_u32(7))",
            "as_u32x(7)",
            "as_field(as_u32(1)+as_u32(2))",
            "as_field(as_u32(1)*as_u32(2))",
            "if 1<2{7}else{9}",
            "if as_u32(0){7}else{9}",
            "if as_u32(7)==7{7}else{9}",
            "as_field(7&as_u32(3))",
            "let mut x=as_u32(7) x=9 as_field(x)",
        ] {
            let source = support::source(body);
            match support::compile_only(&source, caps()) {
                Compilation::Errors(errors) => assert_eq!(errors[0].code, 5, "{body}"),
                other => panic!("{body}: {other:?}"),
            }
            assert!(
                trident::compile(std::str::from_utf8(&source).unwrap(), "oracle.tri").is_err(),
                "seed: {body}"
            );
        }
        for body in [
            "fn f()->U32{7} fn main()->Field{7}",
            "fn f(x:U32)->U32{x} fn main()->Field{as_field(f(7))}",
        ] {
            let source = format!("program sample {body}");
            match support::compile_only(source.as_bytes(), caps()) {
                Compilation::Errors(errors) => assert_eq!(errors[0].code, 5, "{body}"),
                other => panic!("{body}: {other:?}"),
            }
            assert!(trident::compile(&source, "oracle.tri").is_err());
        }
    });
}

#[test]
fn out_of_range_conversion_compiles_then_traps_only_during_program_execution() {
    support::worker(|| {
        for literal in ["4294967296", "18446744069414584320"] {
            let source =
                String::from_utf8(support::source(&format!("as_field(as_u32({literal}))")))
                    .unwrap();
            let bytes = program(&source);
            assert_eq!(
                support::try_run_artifact(&bytes),
                Err("Error(InvZero)".into())
            );
            assert_eq!(
                support::try_rust_value(&source),
                Err("Error(InvZero)".into())
            );
        }
        for body in [
            "fn main()->Field{as_u32(4294967296) 7}",
            "fn main()->Field{let x=as_u32(4294967296) 7}",
            "fn f(x:U32)->Field{7} fn main()->Field{f(as_u32(4294967296))}",
        ] {
            let source = format!("program sample {body}");
            assert_eq!(
                support::try_run_artifact(&program(&source)),
                Err("Error(InvZero)".into())
            );
            assert_eq!(
                support::try_rust_value(&source),
                Err("Error(InvZero)".into())
            );
        }
        // Rejected conversions in unselected branches are never evaluated.
        let source = String::from_utf8(support::source(
            "if false{return as_field(as_u32(4294967296))} 7",
        ))
        .unwrap();
        assert_eq!(support::run_artifact(&program(&source)), 7);
        assert_eq!(support::rust_value(&source), 7);
    });
}

#[test]
fn nested_builtin_expression_and_argument_arenas_respect_sequence_quota() {
    support::worker(|| {
        let source = support::source("as_field(as_u32(7))");
        let mut expected = None;
        for capacity in [2, 3, 4] {
            let mut caps = caps();
            caps[2] = 1;
            caps[3] = capacity;
            match support::compile_only(&source, caps) {
                Compilation::Errors(errors) if capacity == 2 => assert_eq!(errors[0].code, 7),
                Compilation::Program { bytes, .. } if capacity >= 3 => {
                    if let Some(ref expected) = expected {
                        assert_eq!(&bytes, expected);
                    }
                    assert_eq!(support::run_artifact(&bytes), 7);
                    expected = Some(bytes);
                }
                other => panic!("capacity={capacity}: {other:?}"),
            }
        }
    });
}

#[test]
fn conversion_guard_depth_matches_independent_dag_for_shallow_and_deep_arguments() {
    support::worker(|| {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/native_scalar_depth.tri");
        let probe = trident::compile_native_artifact_project(
            &fixture,
            &trident::CompileOptions::default(),
            trident::NativeArtifactProfile::RawNoun,
            trident::NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap();
        for (mode, expected) in [(0, 7), (1, 10)] {
            let generate = |capacity| {
                super::codegen::generate_with_input::<{ 1 << 20 }>(&probe.bytes, |arena| {
                    let cap = support::data::atom(arena, capacity).unwrap();
                    let mode = support::data::atom(arena, mode).unwrap();
                    support::data::pair(arena, cap, mode).unwrap()
                })
            };
            let artifact = generate(4096).unwrap();
            let exact = super::codegen::artifact_depth(&artifact) + 4;
            assert_eq!(generate(exact - 1), Err(7));
            assert_eq!(generate(exact).unwrap(), artifact);
            assert_eq!(generate(exact + 1).unwrap(), artifact);
            assert_eq!(support::run_artifact(&artifact), expected);
        }
    });
}

#[test]
fn conversion_guard_evaluates_its_computed_argument_once() {
    support::worker(|| {
        let source = String::from_utf8(support::source("as_field(as_u32(sub(9,2)))")).unwrap();
        let (value, trace) = support::trace_artifact(&program(&source));
        assert_eq!(value, 7);
        // Subtraction has one row per reached invocation. Re-evaluating the
        // argument for the successful branch would produce a second row.
        assert_eq!(trace.0.iter().filter(|row| row.col(0) == 6).count(), 1);
    });
}
