//! Execute reusable raw-native control flow on the real sequential nox machine.
#[path = "native_control/support.rs"]
mod support;
use support::*;
use trident::{CompileOptions, RAW_ARTIFACT_LIMITS as LIMITS};

#[test]
fn compact_constant_loops_cross_the_old_unrolling_limit() {
    worker(|| {
        let mut sizes = Vec::new();
        for n in [0, 1, 4097, 5000] {
            let source = program(&format!(
                "let mut n: Field = 0\nfor i in 0..{n} {{ n = n + 1 }}\nnox_noun_atom(n)"
            ));
            let artifact = compile(&source);
            let observation = run(&artifact.bytes, 0, 10_000_000, 65536, LIMITS.max_nodes).unwrap();
            assert_eq!(observation.0, n);
            sizes.push(artifact.bytes.len());
            eprintln!(
                "constant {n}: bytes={}, reductions={}, frames={}, nodes={}",
                artifact.bytes.len(),
                observation.1,
                observation.2,
                observation.3
            );
        }
        assert!(
            sizes[1].abs_diff(sizes[3]) < 1024,
            "size follows candidates: {sizes:?}"
        );
        assert!(
            sizes[2].abs_diff(sizes[3]) < 128,
            "size follows candidates: {sizes:?}"
        );
    });
}

#[test]
fn one_dynamic_program_handles_zero_one_and_thousands_of_admitted_candidates() {
    worker(|| {
        let artifact = compile(&program("let end = as_u32(nox_noun_as_field(input))\nlet mut n: Field = 0\nfor i in 0..end bounded 5000 { n = n + 1 }\nnox_noun_atom(n)"));
        for n in [0, 1, 4097, 5000] {
            let observation = run(&artifact.bytes, n, 10_000_000, 65536, LIMITS.max_nodes).unwrap();
            assert_eq!(observation.0, n);
            eprintln!(
                "dynamic {n}: bytes={}, reductions={}, frames={}, nodes={}",
                artifact.bytes.len(),
                observation.1,
                observation.2,
                observation.3
            );
        }
        assert!(run(&artifact.bytes, 5000, 1, 65536, LIMITS.max_nodes).is_err());
        assert_eq!(
            run(&artifact.bytes, 5000, 10_000_000, 64, LIMITS.max_nodes).unwrap_err(),
            "Frames"
        );
        assert!(run(&artifact.bytes, 5000, 10_000_000, 65536, 1000).is_err());
    });
}

#[test]
fn returns_escape_nested_loops_but_only_the_current_function() {
    let source = "program control
fn inner(x: Field) -> Field {
    for i in 0..10 { for j in 0..10 { if as_field(i) == 2 { return x + as_field(j) } } }
    assert_eq(0,1)
    0
}
fn main(input: Noun) -> Noun {
    let mut n: Field = 1
    for i in 0..4 { n = n + inner(2) }
    nox_noun_atom(n)
}";
    assert_eq!(evaluate(source, 0), 9);
    assert_eq!(evaluate(&program("for i in 0..5000 { if as_field(i) == 3 { return nox_noun_atom(42) } }\nassert_eq(0,1)\ninput"),0),42);
}

#[test]
fn mutations_shadowing_and_end_changes_preserve_the_native_loop_contract() {
    let source = program("let mut end: Field = 5\nlet mut n: Field = 0\nfor i in 0..end bounded 9 { n = n * 10 + as_field(i)\nif as_field(i) == 1 { end = 4 }\nlet n: Field = 777\nassert_eq(n,777) }\nnox_noun_atom(n)");
    assert_eq!(evaluate(&source, 0), 123);
    // All three annotations are ignored when outer unrolling used to make i
    // a constant end. The frontend requires an explicit bound syntactically.
    for bound in [0, 1, 3] {
        let source = program(&format!("let mut n: Field = 0\nfor i in 0..3 {{ for j in 0..i bounded {bound} {{ n = n * 10 + as_field(j) + 1 }} }}\nnox_noun_atom(n)"));
        assert_eq!(evaluate(&source, 0), 112);
    }
    let source = program("let mut n: Field = 0\nfor i in 0..3 { for j in i..3 { n = n * 10 + as_field(j) + 1 } }\nnox_noun_atom(n)");
    assert_eq!(evaluate(&source, 0), 123233);
}

#[test]
fn zero_candidates_skip_guard_and_body_and_last_u32_candidate_does_not_wrap() {
    let source = "program control
fn bad() -> Field { assert_eq(0,1)\n0 }
fn main(input: Noun) -> Noun { for i in 0..bad() bounded 0 { assert_eq(0,1) }\ninput }";
    assert_eq!(evaluate(source, 7), 7);
    let source = program("let mut n: Field = 0\nfor i in 4294967295..4294967296 { n = as_field(i) }\nnox_noun_atom(n)");
    assert_eq!(evaluate(&source, 0), u64::from(u32::MAX));
    let source = program("let end: Field = 4294967296\nlet mut n: Field = 0\nfor i in 4294967295..end bounded 1 { n = as_field(i) }\nnox_noun_atom(n)");
    assert_eq!(evaluate(&source, 0), u64::from(u32::MAX));
    for body in [
        "for i in 4294967295..4294967297 { }\ninput",
        "let end: Field = 4294967296\nfor i in 4294967295..end bounded 2 { }\ninput",
    ] {
        assert!(trident::compile_raw_artifact(
            &program(body),
            "bad.tri",
            &CompileOptions::default(),
            LIMITS
        )
        .is_err());
    }
}

#[test]
fn runtime_indices_read_and_rebuild_nested_aggregates() {
    let source = "program control
struct Boxed { cells: [[Field; 3]; 2], marker: Field }
fn main(input: Noun) -> Noun {
    let mut box: Boxed = Boxed { marker: 99, cells: [[1,2,3],[4,5,6]] }
    for i in 0..2 { for j in 0..3 { box.cells[i][j] = box.cells[i][j] + as_field(i) * 10 + as_field(j) } }
    assert_eq(box.marker,99)
    nox_noun_atom(box.cells[0][0] + box.cells[0][2] * 10 + box.cells[1][1] * 100)
}";
    assert_eq!(evaluate(source, 0), 1651);
    let source = program("let mut a = [10,20,30]\nlet i = as_u32(nox_noun_as_field(input))\na[i] = a[i] + 7\nnox_noun_atom(a[0] + a[1] * 10 + a[2] * 100)");
    assert_eq!(evaluate(&source, 1), 3280);
    let bad = source.clone();
    worker(move || {
        assert!(run(&compile(&bad).bytes, 3, 1_000_000, 65536, LIMITS.max_nodes).is_err())
    });
}

#[test]
fn digest_dynamic_indices_agree_with_balanced_destructuring() {
    let source = program("let d = nox_noun_identity(input)\nlet (a,b,c,e) = d\nlet mut total: Field = 0\nfor i in 0..4 { total = total + d[i] }\nassert_eq(total,a+b+c+e)\nnox_noun_atom(46)");
    assert_eq!(evaluate(&source, 0), 46);
}

#[test]
fn balanced_frames_hold_more_than_fifty_eight_live_bindings() {
    let mut body = String::new();
    for i in 0..100 {
        body.push_str(&format!("let v{i}: Field = {i}\n"));
    }
    body.push_str("nox_noun_atom(v0 + v57 + v58 + v99)");
    assert_eq!(evaluate(&program(&body), 0), 214);
}

#[test]
fn diamond_call_graph_is_linear_and_source_recursion_is_rejected() {
    worker(|| {
        let mut source = "program control\nfn f0(x: Field) -> Field { x + 1 }\n".to_string();
        for i in 1..=12 {
            source.push_str(&format!(
                "fn f{i}(x: Field) -> Field {{ f{}(x) + f{}(x) }}\n",
                i - 1,
                i - 1
            ));
        }
        source.push_str(
            "fn main(input: Noun) -> Noun { nox_noun_atom(f12(nox_noun_as_field(input))) }",
        );
        let artifact = compile(&source);
        assert!(artifact.bytes.len() < 100_000, "{}", artifact.bytes.len());
        assert_eq!(
            run(&artifact.bytes, 2, 10_000_000, 65536, LIMITS.max_nodes)
                .unwrap()
                .0,
            12288
        );
        assert_eq!(artifact.bytes, compile(&source).bytes);
        for source in [
            "program p\nfn main(input: Noun) -> Noun { main(input) }",
            "program p\nfn a(x: Noun) -> Noun { b(x) }\nfn b(x: Noun) -> Noun { a(x) }\nfn main(input: Noun) -> Noun { a(input) }",
        ] { assert!(trident::compile_raw_artifact(source,"bad.tri",&CompileOptions::default(),LIMITS).is_err()); }
    });
}

#[test]
fn shadowed_global_constants_do_not_admit_unbounded_runtime_ends() {
    for source in [
        "program p\nconst n: U32 = 9\nfn main(input: Noun) -> Noun { let n: U32 = as_u32(2)\nfor i in 0..n { }\ninput }",
        "program p\nconst n: U32 = 9\nfn f(n: U32) { for i in 0..n { } }\nfn main(input: Noun) -> Noun { input }",
        "program p\nconst n: U32 = 9\nfn main(input: Noun) -> Noun { for n in 0..3 { for i in 0..n { } }\ninput }",
    ] { assert!(trident::check_silent(source,"bad.tri").is_err()); }
}
