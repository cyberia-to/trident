use super::{nouns::data, support};
use data::Noun::Atom;

fn source(body: &str) -> String {
    format!("program sample {body}")
}

fn oracle(source: &str) -> Vec<u8> {
    data::seed(
        &(source.replacen("fn main()->Field", "fn result()->Field", 1)
            + " fn main(input:Noun)->Noun{nox_noun_atom(result())}"),
    )
}

fn agrees(body: &str, expected: u64) {
    let source = source(body);
    assert_eq!(
        support::run_artifact(&data::compile(&source)),
        expected,
        "{source}"
    );
    assert_eq!(
        data::run(&oracle(&source), &Atom(0)).unwrap().bytes,
        Atom(expected).encoded(),
        "seed {source}"
    );
}

fn traps(body: &str, error: &str) {
    let source = source(body);
    assert_eq!(
        support::try_run_artifact(&data::compile(&source)),
        Err(error.into()),
        "{source}"
    );
    assert_eq!(
        data::run(&oracle(&source), &Atom(0)),
        Err(error.into()),
        "seed {source}"
    );
}

#[test]
fn native_assertions_execute_typed_unit_results_and_preserve_final_callable_bindings() {
    support::worker(|| {
        for (body, expected) in [
            ("fn main()->Field{assert(true) assert_eq(7,7) 7}",7),
            ("fn main()->Field{let mut a=assert(true) let b=a a=assert_eq(18446744069414584321,0) b 7}",7),
            ("fn f() {assert_eq(1,1)} fn main()->Field{f() 7}",7),
            ("fn main()->Field{let (a,b)=(assert(true),assert_eq(0,0)) a b 7}",7),
            ("fn main()->Field{assert(true,) assert_eq(7,7,) 7}",7),
            ("fn main()->Field{if false{assert(false)}else{7}}",7),
            ("fn main()->Field{assert(false)} fn assert(c:Bool)->Field{7}",7),
            ("fn assert(c:Bool){} fn main()->Field{assert(false) 7}",7),
            ("fn assert(c:Bool)->Field{1} fn assert(c:Field)->Field{c+2} fn main()->Field{assert(5)}",7),
            ("fn assert_eq(a:Bool,b:Bool)->Field{if a==b{7}else{9}} fn main()->Field{assert_eq(false,false)}",7),
            ("fn main()->Field{let assert=7 let assert_eq=9 assert(true) assert_eq(0,0) assert+assert_eq}",16),
            ("const assert:Field=9 fn main()->Field{assert(true) assert}",9),
            ("fn f(c:Bool)->Field{for i in 0..1{if c{assert(false)}else{return 7}}} fn main()->Field{f(false)}",7),
            ("fn main()->Field{for i in 0..0{assert(false)} 7}",7),
        ] { agrees(body,expected); }
        // An unused assertion function adds no reachable code-table entry.
        assert_eq!(
            data::compile(&source("fn unused(){assert(false)} fn main()->Field{7}")),
            data::compile(&source("fn main()->Field{7}"))
        );
        let input = data::nested();
        data::agrees(
            &data::source("assert(nox_noun_eq(input,input)) assert_eq(7,7) input"),
            &input,
            &input,
        );
    });
}

#[test]
fn native_assertion_failure_compiles_then_traps_and_resolved_false_supplies_coverage() {
    support::worker(|| {
        for body in [
            "fn main()->Field{assert(false)}",
            "fn main()->Field{return assert((false))}",
            "fn main()->Field{assert_eq(0,1) 7}",
            "fn main()->Field{assert(1==0) 7}",
            "fn main()->Field{let x=assert(false) 7}",
            "fn main()->Field{if true{assert(false)} 7}",
            "fn main()->Field{for i in 0..1{assert(false)}}",
            "fn main()->Field{for i in 0..1{assert(false)} 7}",
            "fn main()->Field{for i in 0..1{if true{assert(false)}else{false}}}",
            "fn f(c:Bool)->Field{for i in 0..1{if c{assert(false)}else{return 7}}} fn main()->Field{f(true)}",
            "fn f(c:Bool)->Field{if c{assert(false)}else{assert(false)}} fn main()->Field{f(false)}",
            "fn stop(){assert(false)} fn main()->Field{stop() 7}",
            "fn main()->Field{for i in 0..1{for j in 0..1{assert(false)}}}",
        ] { traps(body,"Error(InvZero)"); }
        for ty in [
            "Field",
            "Bool",
            "U32",
            "Noun",
            "Digest",
            "(Field,Bool)",
            "[Field;2]",
            "R",
        ] {
            for value in ["assert(false)", "return assert((false))"] {
                traps(
                    &format!(
                        "struct R{{a:Field}} fn f()->{ty}{{{value}}} fn main()->Field{{f() 7}}"
                    ),
                    "Error(InvZero)",
                );
            }
        }
    });
}

#[test]
fn native_assertions_reject_wrong_types_arity_and_false_halting_claims() {
    support::worker(|| {
        for body in [
            "fn main()->Field{assert(0) 7}",
            "fn main()->Field{assert(as_u32(0)) 7}",
            "fn main()->Field{assert() 7}",
            "fn main()->Field{assert(true,false) 7}",
            "fn main()->Field{assert_eq(0) 7}",
            "fn main()->Field{assert_eq(0,0,0) 7}",
            "fn main()->Field{assert_eq(true,true) 7}",
            "fn main()->Field{assert_eq(as_u32(0),as_u32(0)) 7}",
            "fn main()->Field{assert_eq(nox_noun_atom(0),nox_noun_atom(0)) 7}",
            "fn main()->Field{assertx(true) 7}",
            "fn main()->Field{assert_eqe(0,0) 7}",
            "fn main()->Field{assert(true)}",
            "fn main()->Field{assert(1==0)}",
            "fn main()->Field{return assert_eq(0,1)}",
            "fn main()->Field{for i in 0..0{assert(false)}}",
            "fn main()->Field{let x=assert(false)}",
            "fn main()->Field{assert(false) 7}",
            "fn main()->Field{assert(false) let x=7}",
            "fn main()->Field{assert(false) return 7}",
            "fn main()->Field{if true{assert(false)}else{false}}",
            "fn assert(c:Bool){} fn main()->Field{assert(false)}",
            "fn stop(){assert(false)} fn main()->Field{stop()}",
            "fn f(c:Bool)->Field{if c{assert(false)}} fn main()->Field{f(true)}",
            "fn f(c:Bool)->Field{for i in 0..1{if c{assert(false)}else{7}}} fn main()->Field{f(false)}",
            "fn main()->Field{let f=false assert(f)}",
        ] {
            let source=source(body);
            match support::compile_only(source.as_bytes(),data::caps()) {
                support::Compilation::Errors(errors)=>assert_eq!(errors[0].code,5,"{source}"),
                other=>panic!("{source}: {other:?}"),
            }
            assert!(trident::compile(&source,"oracle.tri").is_err(),"seed {source}");
        }
    });
}

#[test]
fn native_assertions_evaluate_arguments_once_in_left_to_right_order() {
    support::worker(|| {
        for (expression, count) in [
            ("assert(sub(9,2)==7)", 1),
            ("assert_eq(sub(9,2),sub(10,3))", 2),
        ] {
            let source = source(&format!("fn main()->Field{{{expression} 7}}"));
            let (value, trace) = support::trace_artifact(&data::compile(&source));
            assert_eq!(value, 7);
            assert_eq!(trace.0.iter().filter(|row| row.col(0) == 6).count(), count);
        }
        let a = "as_field(as_u32(4294967296))";
        let b = "nox_noun_as_field(nox_noun_head(nox_noun_atom(0)))";
        traps(
            &format!("fn main()->Field{{assert_eq({a},{b}) 7}}"),
            "Error(InvZero)",
        );
        traps(
            &format!("fn main()->Field{{assert_eq({b},{a}) 7}}"),
            "Error(AxisError)",
        );
    });
}

#[test]
fn native_assertion_nodes_and_argument_links_respect_exact_cap() {
    support::worker(|| {
        let source = source("fn main()->Field{assert_eq(7,7) 9}");
        let mut previous = None;
        for cap in [3, 4, 5] {
            let mut caps = data::caps();
            caps[2] = 1;
            caps[3] = cap;
            match support::compile_only(source.as_bytes(), caps) {
                support::Compilation::Errors(errors) if cap == 3 => assert_eq!(errors[0].code, 7),
                support::Compilation::Program { bytes, .. } if cap >= 4 => {
                    assert_eq!(support::run_artifact(&bytes), 9);
                    if let Some(ref old) = previous {
                        assert_eq!(old, &bytes);
                    }
                    previous = Some(bytes);
                }
                other => panic!("cap={cap}: {other:?}"),
            }
        }
    });
}

#[test]
fn native_assertion_guards_count_both_arms_in_complete_artifact_depth() {
    support::worker(|| {
        let probe = trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_digest_depth.tri"),
            &trident::CompileOptions::default(),
            trident::NativeArtifactProfile::RawNoun,
            trident::NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap()
        .bytes;
        for expression in [
            "assert(true)",
            "assert_eq(7,7)",
            "assert(sub(sub(sub(sub(sub(sub(28,1),2),3),4),5),6)==7)",
            "assert_eq(sub(sub(sub(sub(sub(sub(28,1),2),3),4),5),6),7)",
            "assert_eq(7,sub(sub(sub(sub(sub(sub(28,1),2),3),4),5),6))",
        ] {
            let source = source(&format!("fn main()->Field{{{expression} 7}}"));
            let generate = |cap| {
                super::codegen::generate_with_input::<{ 1 << 20 }>(&probe, |arena| {
                    let source = support::data::Bytes::from_slice(arena, source.as_bytes(), 4096)
                        .unwrap()
                        .encode(arena)
                        .unwrap();
                    let cap = support::data::atom(arena, cap).unwrap();
                    support::data::pair(arena, cap, source).unwrap()
                })
            };
            let bytes = generate(4096).unwrap();
            let exact = super::codegen::artifact_depth(&bytes) + 4;
            assert_eq!(generate(exact - 1), Err(7));
            assert_eq!(generate(exact).unwrap(), bytes);
            assert_eq!(generate(exact + 1).unwrap(), bytes);
            assert_eq!(support::run_artifact(&bytes), 7);
        }
    });
}
