use super::support::{self, Result};

#[test]
fn local_slots_preserve_snapshots_mutation_and_shadowing() {
    support::worker(|| {
        let c1 = support::compiler().to_vec();
        for (body, expected) in [
            ("let x: Field = 7 x", 7),
            ("let x=7 9", 9),
            ("let x=7 (x+1)", 8),
            ("let x=7 let y=x\n(x+1)", 8),
            ("let x=7 let y=x// comment\r inside\n(x+1)", 8),
            ("let mut x=7 x=8 9", 9),
            ("let x = 2+3 let y: Field = x*x y", 25),
            ("let mut x = 7 let y = x x = 9 y*100+x", 709),
            ("let x = 7 let x = x+1 x", 8),
            ("let mut x: Field = 2 x = x+3 x = x*x x", 25),
            ("let common_prefix_a = 3 let common_prefix_b = 7 common_prefix_a*10+common_prefix_b", 37),
            ("let a=1 let b=2 let c=3 let d=4 let e=5 a+b*10+c*100+d*1000+e*10000", 54321),
            ("let mut a=1 let b=2 let c=3 a=c+b a*100+b*10+c", 523),
            ("let a=1 let b=2 let c=3 let d=4 let mut e=5 e=a+c a+b+c+d+e", 14),
        ] {
            let source = support::source(body);
            match support::compile(&source) {
                Result::Program { value, .. } => {
                    assert_eq!(value, expected, "{body}");
                    assert_eq!(support::rust_value(std::str::from_utf8(&source).unwrap()), expected, "{body}");
                }
                other => panic!("{body}: {other:?}"),
            }
        }
        assert_eq!(c1, support::compiler());
    });
}

#[test]
fn local_errors_bind_the_offending_name_and_preserve_mutability() {
    support::worker(|| {
        for (body, name) in [
            ("let x = x x", "x"),
            ("let x = 7 missing", "missing"),
            ("let x = 7 x = 8 x", "x"),
            ("missing = 7 missing", "missing"),
            ("let mut x = 7 let x = x+1 x = 9 x", "x"),
            ("let common_prefix_a=1 common_prefix_b", "common_prefix_b"),
        ] {
            let source = support::source(body);
            let error = support::error(&source, 5);
            assert_eq!(
                &source[error.start as usize..error.end as usize],
                name.as_bytes(),
                "{body}"
            );
        }
        for body in [
            "let x = }",
            "let = 1 x",
            "let mut = 1",
            "let x: = 1 x",
            "let x 1 x",
            "let mut x=1 x=",
        ] {
            support::error(&support::source(body), 2);
        }
        for body in ["let x=7 x(1)", "let x=7 x\r(1)", "missing(1)"] {
            support::error(&support::source(body), 5);
        }
        for body in ["let x: U32 = 1 x", "let x: Other=1 x"] {
            support::error(&support::source(body), 6);
        }
    });
}

#[test]
fn expression_records_obey_exact_sequence_caps_without_changing_artifacts() {
    support::worker(|| {
        let source = support::source("let mut x=7 let y=x x=9 y*100+x");
        let mut expected = None;
        // literal7, local x, literal9, local y, literal100, multiply, local x, add.
        for cap in [7, 8, 16, 4096] {
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
                Result::Errors(errors) if cap == 7 => assert_eq!(errors[0].code, 7),
                Result::Program { bytes, value, .. } if cap >= 8 => {
                    assert_eq!(value, 709);
                    if let Some(ref previous) = expected {
                        assert_eq!(previous, &bytes);
                    }
                    expected = Some(bytes);
                }
                other => panic!("cap={cap}: {other:?}"),
            }
        }
    });
}

#[test]
fn locals_cross_parser_and_emitter_chunks() {
    support::worker(|| {
        for length in [7, 8, 9] {
            let body = format!("let mut x=0 {}x", "x=x+1 ".repeat(length));
            let source = support::source(&body);
            match support::compile(&source) {
                Result::Program {
                    value,
                    reductions,
                    nodes,
                    frames,
                    ..
                } => {
                    assert_eq!(value, length as u64);
                    eprintln!("assignments={length} reductions={reductions} nodes={nodes} frames={frames}");
                }
                other => panic!("{length}: {other:?}"),
            }
        }
    });
}
