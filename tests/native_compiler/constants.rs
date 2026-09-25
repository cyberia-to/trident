use super::{nouns::data, support};
use data::Noun::Atom;

fn agrees(body: &str, expected: u64) {
    let source = format!("program sample {body}");
    let program = data::compile(&source);
    assert_eq!(support::run_artifact(&program), expected, "{source}");
    let oracle = source.replacen("fn main()->Field", "fn result()->Field", 1)
        + " fn main(input:Noun)->Noun{nox_noun_atom(result())}";
    assert_eq!(
        data::run(&data::seed(&oracle), &Atom(0)).unwrap().bytes,
        Atom(expected).encoded(),
        "seed {source}"
    );
}

#[test]
fn native_constants_freeze_final_typed_bindings_before_all_function_bodies() {
    support::worker(|| {
        for (body, expected) in [
            ("fn main()->Field{A} const A:Field=7", 7),
            ("const A:Field=B const B:Field=((7)) fn main()->Field{A}", 7),
            (
                "const A:Field=2 fn f()->Field{A} const A:Field=7 fn main()->Field{f()+A}",
                14,
            ),
            (
                "const A:Field=B const B:Field=2 const B:Field=7 fn main()->Field{A}",
                7,
            ),
            ("const A:Field=A const A:Field=7 fn main()->Field{A}", 7),
            (
                "pub const A:U32=7 const A:Field=4294967296 fn main()->Field{A}",
                4294967296,
            ),
            (
                "const A:U32=B const B:U32=4294967295 fn main()->Field{as_field(A)}",
                4294967295,
            ),
            (
                "const A:U32=((0004294967295)) fn main()->Field{as_field(A & as_u32(7))}",
                7,
            ),
            ("const A:Field=18446744069414584321 fn main()->Field{A}", 0),
            ("const A:Field=18446744069414584322 fn main()->Field{A}", 1),
            (
                "const A:Field=18446744073709551615 fn main()->Field{A}",
                4294967294,
            ),
            (
                "const A:Field=7 fn f(A:Field)->Field{A} fn main()->Field{let A=3 f(A)}",
                3,
            ),
            ("const A:Field=7 fn main()->Field{let(A,b)=(3,4) A+b}", 7),
            ("const A:Field=7 fn main()->Field{let mut A=A A=9 A}", 9),
            (
                "const I:Field=18446744069414584321 fn main()->Field{[7][I]}",
                7,
            ),
            ("const I:U32=0 fn main()->Field{[7][I]}", 7),
            ("const C:Field=0 fn main()->Field{if C{return 7}}", 7),
            (
                "const C:Field=18446744069414584321 fn main()->Field{if C{return 7}}",
                7,
            ),
            ("const C:Field=1 fn main()->Field{if C{}else{return 7}}", 7),
        ] {
            agrees(body, expected);
        }
        let long = "common_prefix_".to_owned() + &"a".repeat(256);
        agrees(&format!("const {long}a:Field=3 const {long}b:Field=7 fn main()->Field{{{long}a*10+{long}b}}"),37);
        // Constants introduce no code table entry or expression wrapper.
        let literal = data::compile("program sample fn main()->Field{7}");
        for body in [
            "const A:Field=7 fn main()->Field{A}",
            "const A:Field=7 fn main()->Field{7}",
            "const A:Field=(B) const B:Field=7 fn main()->Field{A}",
        ] {
            assert_eq!(data::compile(&format!("program sample {body}")), literal);
        }
    });
}

#[test]
fn native_constants_validate_replaced_and_unused_initializers_without_partial_acceptance() {
    support::worker(|| {
        for (decl, code) in [
            ("const A:Field=missing", 5),
            ("const A:Field=missing const A:Field=7", 5),
            ("const A:Field=7 const A:Field=missing", 5),
            ("const A:Field=B const B:Field=A", 5),
            ("const A:Field=A", 5),
            ("const A:Field=B const B:U32=7", 5),
            ("const A:U32=B const B:Field=7", 5),
            ("const A:U32=4294967296 const A:U32=7", 5),
            ("const A:U32=18446744069414584321", 5),
            ("const A:Field=7+2", 5),
            ("const A:Field=(7*2)", 5),
            ("const A:Field=7==2", 5),
            ("const A:U32=7 & 2", 5),
            ("const A:Field=f() fn f()->Field{7}", 5),
            ("const A:Field=18446744073709551616", 1),
            ("const A:Field=(7", 2),
            ("const A:Field=7)", 2),
            ("const A Field=7", 2),
            ("const A:Field 7", 2),
            ("const A:Field=7 8", 2),
            ("const A:Field=other.A", 6),
        ] {
            let source = format!("program sample {decl} fn main()->Field{{7}}");
            match support::compile_only(source.as_bytes(), data::caps()) {
                support::Compilation::Errors(errors) => {
                    assert_eq!(errors[0].code, code, "{source}");
                    assert!(
                        errors[0].start <= errors[0].end && errors[0].end <= source.len() as u32
                    );
                }
                other => panic!("{source}: {other:?}"),
            }
            assert!(
                trident::compile(&source, "oracle.tri").is_err(),
                "seed {source}"
            );
        }
        for (body, code) in [
            ("const A:Field=7 fn main()->Field{A=9 A}", 5),
            (
                "const C:Field=0 fn main()->Field{let C=1 if C{return 7}}",
                5,
            ),
            ("const A:Field=7 fn main()->Field{let (A,b)=(1,2) A=9 A}", 5),
            ("const A:U32=0 fn main()->Field{if A {7}else{9}}", 5),
            ("const A:Field=1 fn f(a:[Field;A]){} fn main()->Field{7}", 6),
            ("const A:Field=1 fn main()->Field{for i in 0..A{} 7}", 6),
            ("fn main()->Field{const A:Field=7 A}", 6),
            (
                "const A:Field=0 fn main()->Field{if A{return 7}else{false}}",
                5,
            ),
        ] {
            let source = format!("program sample {body}");
            match support::compile_only(source.as_bytes(), data::caps()) {
                support::Compilation::Errors(errors) => {
                    assert_eq!(errors[0].code, code, "{source}")
                }
                other => panic!("{source}: {other:?}"),
            }
        }
        let source = "program sample const I:Field=1 fn main()->Field{[7][I]}";
        let program = data::compile(source);
        assert!(support::try_run_artifact(&program)
            .unwrap_err()
            .contains("InvZero"));
    });
}

#[test]
fn native_constants_keep_previous_default_arena_and_artifacts() {
    support::worker(|| {
        for (name, source, expected) in [
            (
                "body-chunks",
                support::source(&format!("let mut x=0 {}x", "x=x+1 ".repeat(9))),
                9,
            ),
            (
                "stack64",
                support::source(&format!("{}1{}", "(".repeat(64), ")".repeat(64))),
                1,
            ),
        ] {
            let result = support::try_compile_package(
                &[support::module(&source)],
                "sample",
                "main",
                support::options(),
                support::CAPS,
            );
            if let Ok(support::Result::Program {
                reductions,
                nodes,
                frames,
                ..
            }) = &result
            {
                eprintln!("{name}: {reductions} reductions {nodes} nodes {frames} frames");
            }
            if result.is_err() {
                // Diagnostic measurement only; the original default-cap assertion still fails.
                if let support::Compilation::Program {
                    reductions,
                    nodes,
                    frames,
                    ..
                } = support::compile_only(&source, data::caps())
                {
                    eprintln!(
                        "excess {name}: {reductions} reductions {nodes} nodes {frames} frames"
                    );
                }
            }
            assert_eq!(support::value(result.unwrap()), expected);
        }
    });
}
