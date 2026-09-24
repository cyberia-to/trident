use super::{codegen, support};
use support::Compilation;

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

fn source(body: &str) -> String {
    String::from_utf8(support::source(body)).unwrap()
}

#[test]
fn literal_loops_preserve_scope_mutation_and_repeated_initialization() {
    support::worker(|| {
        for (body, expected) in [
            ("let mut x=7 for i in 0..0{x=9} x", 7),
            ("let mut x=7 for i in 9..2{x=9} x", 7),
            ("let mut x=0 for i in 2..5{x=x*10+as_field(i)} x", 234),
            ("let mut x=0 for i in 0002..0005{x=x+as_field(i)} x", 9),
            ("let i=7 let mut x=0 for i in 0..3{x=x+as_field(i)} x+i", 10),
            ("let mut x=0 for i in 0..3{let mut y=2 y=y+as_field(i) x=x+y} x", 9),
            ("let mut x=0 for i in 0..3{for i in 1..3{x=x+as_field(i)} x=x+as_field(i)*10} x", 39),
            ("for i in 0..1{7} 9", 9),
            ("if true{for i in 0..2{if as_field(i)==1{return 7}}}else{for i in 0..1{return 9}} 3", 7),
            ("let mut x=0 if false{x=9}else if true{for i in 0..2{x=x+1}} x", 2),
        ] {
            let source = source(body);
            assert_eq!(support::run_artifact(&program(&source)), expected, "{source}");
            assert_eq!(support::rust_value(&source), expected, "seed: {source}");
        }
    });
}

#[test]
fn loop_returns_escape_iterations_and_only_the_current_function() {
    support::worker(|| {
        for (body, expected) in [
            ("fn main()->Field{for i in 0..3{for j in 0..3{if as_field(i)==1{return as_field(j)+7}}} as_field(as_u32(4294967296))}", 7),
            ("fn f(x:Field)->Field{for i in 0..3{return x+as_field(i)}} fn main()->Field{let mut x=0 for i in 0..4{x=x+f(2)} x}", 8),
            ("fn main()->Field{for i in 0..1{return 7} as_field(as_u32(4294967296))}", 7),
            ("fn main()->Field{for i in 0..1{if true{return 7}else{return 9}}}", 7),
            ("fn main()->Field{for i in 0..3{if as_field(i)==0{return 7} as_u32(4294967296)} 9}", 7),
            ("fn f(){for i in 0..3{return}} fn main()->Field{for i in 0..2{f()} 7}", 7),
        ] {
            let source = format!("program sample {body}");
            assert_eq!(support::run_artifact(&program(&source)), expected, "{source}");
            assert_eq!(support::rust_value(&source), expected, "seed: {source}");
        }
    });
}

#[test]
fn raw_loop_bounds_and_empty_bodies_are_checked_before_emission() {
    support::worker(|| {
        for body in ["for i in 0..{} 7", "for i in ..1{} 7", "for i in 0..1"] {
            match support::compile_only(source(body).as_bytes(), caps()) {
                Compilation::Errors(errors) => assert_eq!(errors[0].code, 2, "{body}"),
                other => panic!("{body}: {other:?}"),
            }
        }
        for body in [
            "for i in true..1{} 7",
            "for i in 0..false{} 7",
            "for i in 0..1{7}",
            "for i in 0..0{return 7}",
            "for i in 0..0{return false} 7",
            "for i in 0..0{missing} 7",
            "for i in 0..2{i=as_u32(0)} 7",
            "for i in 0..2{let x=7} x",
            "for i in 0..2{} as_field(i)",
            "for i in 0..4294967297{} 7",
            "for i in 4294967296..4294967296{} 7",
            "for i in 0..18446744069414584321{} 7",
            "for i in 18446744069414584322..0{} 7",
            "for i in 4294967295..4294967296{return as_field(i)}",
        ] {
            let source = source(body);
            match support::compile_only(source.as_bytes(), caps()) {
                Compilation::Errors(errors) => assert_eq!(errors[0].code, 5, "{source}"),
                other => panic!("{source}: {other:?}"),
            }
        }
        for body in [
            "let n=3 for i in 0..n bounded 3{} 7",
            "for i in 0..3{for j in 0..i{}} 7",
            "for i in 0..3 bounded 3{} 7",
        ] {
            match support::compile_only(source(body).as_bytes(), caps()) {
                Compilation::Errors(errors) => assert_eq!(errors[0].code, 6, "{body}"),
                other => panic!("{body}: {other:?}"),
            }
        }
        let source = source("for i in 4294967295..4294967296{return as_field(i)} 7");
        assert_eq!(support::run_artifact(&program(&source)), 4294967295);
    });
}

#[test]
fn reusable_loop_artifacts_cross_the_seed_flat_unrolling_limit() {
    support::worker(|| {
        let mut sizes = Vec::new();
        for count in [0, 1, 4097, 5000] {
            let body = format!("let mut x=0 for i in 0..{count}{{x=x+1}} x");
            let bytes = program(&source(&body));
            let result = support::native::run(&bytes, 0, 10_000_000, 65536, 196608).unwrap();
            assert_eq!(result.0, count);
            sizes.push(bytes.len());
            // Adapt only the raw entry/result ABI; the loop body is identical.
            let seed = support::native::compile(&support::native::program(&format!(
                "let mut x=0 for i in 0..{count}{{x=x+1}} nox_noun_atom(x)"
            )));
            let oracle = support::native::run(&seed.bytes, 0, 10_000_000, 65536, 196608).unwrap();
            assert_eq!(oracle.0, count);
            eprintln!(
                "guest loop {count}: bytes={}, reductions={}, frames={}, nodes={}",
                bytes.len(),
                result.1,
                result.2,
                result.3
            );
            if count == 5000 {
                assert!(support::native::run(&bytes, 0, 1, 65536, 196608).is_err());
                assert_eq!(
                    support::native::run(&bytes, 0, 10_000_000, 64, 196608),
                    Err("Frames".into())
                );
                assert!(support::native::run(&bytes, 0, 10_000_000, 65536, 1000).is_err());
                assert_eq!(
                    support::native::run(&bytes, 0, result.1, result.2, result.3).unwrap(),
                    result
                );
                assert!(support::native::run(&bytes, 0, result.1 - 1, result.2, result.3).is_err());
                assert_eq!(
                    support::native::run(&bytes, 0, result.1, result.2 - 1, result.3),
                    Err("Frames".into())
                );
                assert!(support::native::run(&bytes, 0, result.1, result.2, result.3 - 1).is_err());
            }
        }
        assert!(sizes[1].abs_diff(sizes[3]) < 1024, "{sizes:?}");
        assert!(sizes[2].abs_diff(sizes[3]) < 128, "{sizes:?}");
        // A maximum-length admitted range can still return in its first candidate.
        let bytes = program(&source("for i in 0..4294967296{return as_field(i)+7} 9"));
        assert_eq!(support::run_artifact(&bytes), 7);
    });
}

#[test]
fn loop_slots_and_combined_code_entries_obey_the_sequence_allowance() {
    support::worker(|| {
        for (source, exact) in [
            ("program sample fn main()->Field{for i in 0..1{return 7}}", 2),
            ("program sample fn f()->Field{for i in 0..1{return 7}} fn main()->Field{for i in 0..1{return f()}}", 4),
        ] {
            let mut expected = None;
            for capacity in [exact-1,exact,exact+1] {
                let mut limits = caps(); limits[2]=1; limits[3]=capacity;
                match support::compile_only(source.as_bytes(),limits) {
                    Compilation::Errors(errors) if capacity<exact => assert_eq!(errors[0].code,7,"{source}"),
                    Compilation::Program{bytes,..} if capacity>=exact => {
                        assert_eq!(support::run_artifact(&bytes),7);
                        if let Some(ref expected) = expected { assert_eq!(&bytes,expected); }
                        expected=Some(bytes);
                    }
                    other=>panic!("capacity={capacity}: {source}: {other:?}"),
                }
            }
        }
    });
}

#[test]
fn loop_table_ownership_is_independent_of_function_discovery_order() {
    support::worker(|| {
        let f = "fn f(x:Field)->Field{let mut n=x for i in 0..3{n=n+as_field(i)} n}";
        let g = "fn g(x:Field)->Field{let mut n=0 for i in 0..2{for j in 0..2{n=n+f(x)+as_field(j)}} n}";
        let main = "fn main()->Field{g(7)}";
        let mut expected = None;
        for body in [
            format!("{f} {g} {main}"),
            format!("{main} {g} {f}"),
            format!("fn unused()->Field{{for i in 0..1{{return 9}}}} {main} {f} {g}"),
        ] {
            let bytes = program(&format!("program sample {body}"));
            assert_eq!(support::run_artifact(&bytes), 42);
            if let Some(ref expected) = expected {
                assert_eq!(&bytes, expected);
            }
            expected = Some(bytes);
        }
    });
}

#[test]
fn complete_loop_table_depth_matches_the_independent_artifact_reader() {
    support::worker(|| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/native_loop_depth.tri");
        let probe = trident::compile_native_artifact_project(
            &path,
            &trident::CompileOptions::default(),
            trident::NativeArtifactProfile::RawNoun,
            trident::NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap();
        let generate = |cap| {
            codegen::generate_with_input::<{ 1 << 20 }>(&probe.bytes, |arena| {
                support::data::atom(arena, cap).unwrap()
            })
        };
        let bytes = generate(4096).unwrap();
        assert_eq!(support::run_artifact(&bytes), 6);
        let exact = codegen::artifact_depth(&bytes) + 4;
        assert_eq!(generate(exact - 1), Err(7));
        assert_eq!(generate(exact).unwrap(), bytes);
        assert_eq!(generate(exact + 1).unwrap(), bytes);
    });
}
