use super::{codegen, nouns::data, support};
use data::{Noun, Noun::Atom};

fn words(input: &Noun) -> [u64; 4] {
    let bytes = input.encoded();
    std::array::from_fn(|i| u64::from_le_bytes(bytes[8 + i * 8..16 + i * 8].try_into().unwrap()))
}
fn balanced(w: [u64; 4]) -> Noun {
    Noun::pair(
        Noun::pair(Atom(w[0]), Atom(w[1])),
        Noun::pair(Atom(w[2]), Atom(w[3])),
    )
}
fn output() -> &'static str {
    "nox_noun_pair(nox_noun_pair(nox_noun_atom(d[0]),nox_noun_atom(d[1])),nox_noun_pair(nox_noun_atom(d[2]),nox_noun_atom(d[3])))"
}

#[test]
fn digest_identity_preserves_all_four_particle_words_through_locals_calls_and_loops() {
    support::worker(|| {
        let source = data::source(&format!(
            "let d:Digest=nox_noun_identity(input) {}",
            output()
        ));
        let sources = [
            source,
            data::source(&format!("let mut d=nox_noun_identity(nox_noun_atom(0)) for i in 0..3{{d=nox_noun_identity(input)}} {}",output())),
            format!("program sample fn take(d:Digest)->Digest{{d}} fn make(x:Noun)->Digest{{nox_noun_identity(x)}} fn main(input:Noun)->Noun{{let d=take(make(input)) {}}}",output()),
            data::source(&format!("let d=nox_noun_identity(input) let mut copy=d copy=nox_noun_identity(nox_noun_atom(0)) {}",output())),
        ];
        for source in sources {
            let bytes = data::compile(&source);
            let seed = data::seed(&source);
            for input in [
                Atom(0),
                Atom(18446744069414584320),
                data::nested(),
                Noun::pair(Atom(1), Atom(2)),
            ] {
                let expected = balanced(words(&input)).encoded();
                assert_eq!(
                    data::run(&bytes, &input).unwrap().bytes,
                    expected,
                    "{source}"
                );
                assert_eq!(
                    data::run(&seed, &input).unwrap().bytes,
                    expected,
                    "seed {source}"
                );
            }
        }
    });
}

#[test]
fn digest_postfix_delimiters_precedence_and_dynamic_field_u32_indices_match_seed() {
    support::worker(|| {
        let input = data::nested();
        let w = words(&input);
        for (body, expected) in [
            (
                "let d=nox_noun_identity(input) nox_noun_atom(d[0]+d[1]*0)",
                w[0],
            ),
            (
                "let d=nox_noun_identity(input) nox_noun_atom(d[as_u32(sub(3,1))])",
                w[2],
            ),
            (
                "let d=nox_noun_identity(input) nox_noun_atom((d)[(1+2)])",
                w[3],
            ),
            ("nox_noun_atom(nox_noun_identity(input)\n[0])", w[0]),
            ("let d=nox_noun_identity(input) nox_noun_atom(d\n[1])", w[1]),
            (
                "let d=nox_noun_identity(input) nox_noun_atom(d[sub(d[0],d[0])+2])",
                w[2],
            ),
            (
                "let d=nox_noun_identity(input) nox_noun_atom(d[18446744069414584321])",
                w[0],
            ),
        ] {
            data::agrees(&data::source(body), &input, &Atom(expected));
        }
        for body in [
            "fn f(x:Digest)->Digest{x} fn main(input:Noun)->Noun{let d=f(nox_noun_identity(input)) nox_noun_atom(f(d)[sub(3,1)])}",
            "fn f(a:Field,b:Field)->Field{a+b*0} fn main(input:Noun)->Noun{let d=nox_noun_identity(input) nox_noun_atom(f(d[2],d[3]))}",
        ] { data::agrees(&format!("program sample {body}"),&input,&Atom(w[2])); }
        for body in [
            "let d=nox_noun_identity(input) if d==nox_noun_identity(input){input}else{nox_noun_atom(0)}",
            "let d=nox_noun_identity(input) if d==nox_noun_identity(nox_noun_atom(0)){nox_noun_atom(0)}else{input}",
        ] { data::agrees(&data::source(body),&input,&input); }
        let source="program sample fn nox_noun_identity(x:Noun)->Noun{x} fn main(input:Noun)->Noun{nox_noun_identity(input)}";
        data::agrees(source, &input, &input);
    });
}

#[test]
fn digest_type_errors_and_malformed_index_delimiters_never_publish_a_program() {
    support::worker(|| {
        // The same raw-profile oracle accepts a well-typed structured entry.
        assert!(trident::compile_raw_artifact(
            &data::source("nox_noun_atom(nox_noun_identity(input)[0])"),
            "oracle.tri",
            &trident::CompileOptions::default(),
            trident::NATIVE_ARTIFACT_LIMITS
        )
        .is_ok());
        for (tail, code) in [
            ("let x:Digest=input input", 5),
            ("let x:Noun=d input", 5),
            ("nox_noun_identity(7) input", 5),
            ("nox_noun_identity() input", 5),
            ("nox_noun_identity(input,input) input", 5),
            ("nox_noun_identitx(input) input", 5),
            ("d+d input", 5),
            ("d*d input", 5),
            ("d<d input", 5),
            ("d&d input", 5),
            ("d==input input", 5),
            ("if d{return input} input", 5),
            ("d[true] input", 5),
            ("d[input] input", 5),
            ("d[d] input", 5),
            ("input[0] input", 5),
            ("d[0][0] input", 5),
            ("d[] input", 2),
            ("d[1,2] input", 2),
            ("d[(1] input", 2),
            ("d[sub(3,1] input", 2),
            ("d[1) input", 2),
            ("d[1 input", 2),
            ("d[0]=7 input", 2),
        ] {
            let source = data::source(&format!("let d=nox_noun_identity(input) {tail}"));
            match support::compile_only(source.as_bytes(), data::caps()) {
                support::Compilation::Errors(errors) => {
                    assert_eq!(errors[0].code, code, "{source}")
                }
                other => panic!("{source}: {other:?}"),
            }
            assert!(
                trident::compile_raw_artifact(
                    &source,
                    "oracle.tri",
                    &trident::CompileOptions::default(),
                    trident::NATIVE_ARTIFACT_LIMITS
                )
                .is_err(),
                "seed {source}"
            );
        }
        for body in [
            "fn main()->Digest{nox_noun_identity(nox_noun_atom(0))}",
            "fn main(input:Digest)->Noun{nox_noun_atom(input[0])}",
        ] {
            match support::compile_only(format!("program sample {body}").as_bytes(), data::caps()) {
                support::Compilation::Errors(errors) => assert_eq!(errors[0].code, 3),
                other => panic!("{other:?}"),
            }
        }
    });
}

#[test]
fn digest_indices_trap_at_execution_including_discarded_and_unused_values() {
    support::worker(|| {
        for body in [
            "let d=nox_noun_identity(input) nox_noun_atom(d[4])",
            "let d=nox_noun_identity(input) nox_noun_atom(d[18446744069414584320])",
            "let d=nox_noun_identity(input) nox_noun_atom(d[as_u32(4294967295)])",
            "let d=nox_noun_identity(input) d[4] input",
            "let d=nox_noun_identity(input) let ignored=d[4] input",
        ] {
            let source = data::source(body);
            for bytes in [data::compile(&source), data::seed(&source)] {
                assert_eq!(
                    data::run(&bytes, &data::nested()),
                    Err("Error(InvZero)".into()),
                    "{source}"
                );
            }
        }
        let source="program sample fn f(x:Field,y:Noun)->Noun{y} fn main(input:Noun)->Noun{f(nox_noun_identity(input)[4],input)}";
        for bytes in [data::compile(source), data::seed(source)] {
            assert_eq!(
                data::run(&bytes, &data::nested()),
                Err("Error(InvZero)".into())
            );
        }
        data::agrees(
            &data::source("if false{nox_noun_identity(input)[4]} input"),
            &data::nested(),
            &data::nested(),
        );
    });
}

#[test]
fn digest_base_and_index_execute_once_in_source_order_with_exact_runtime_quotas() {
    support::worker(|| {
        let source =
            data::source("nox_noun_atom(nox_noun_identity(nox_noun_atom(sub(90,11)))[sub(3,1)])");
        let bytes = data::compile(&source);
        let mut trace = nox::trace::VecTrace::default();
        let run = data::run_traced(&bytes, &Atom(0), 1_000_000, 65536, 196608, &mut trace).unwrap();
        assert_eq!(run.bytes, Atom(words(&Atom(79))[2]).encoded());
        assert_eq!(trace.0.iter().filter(|row| row.col(0) == 6).count(), 2);
        let exact = data::run(&bytes, &Atom(0)).unwrap();
        assert_eq!(
            data::run_traced(
                &bytes,
                &Atom(0),
                exact.reductions,
                exact.frames,
                exact.nodes,
                &mut nox::NoTrace
            )
            .unwrap(),
            exact
        );
        for (budget, frames, nodes) in [
            (exact.reductions - 1, exact.frames, exact.nodes),
            (exact.reductions, exact.frames - 1, exact.nodes),
            (exact.reductions, exact.frames, exact.nodes - 1),
        ] {
            assert!(
                data::run_traced(&bytes, &Atom(0), budget, frames, nodes, &mut nox::NoTrace)
                    .is_err()
            );
        }
        for (body,error) in [
            ("nox_noun_atom(nox_noun_identity(nox_noun_head(nox_noun_atom(7)))[nox_noun_as_field(input)])","Error(AxisError)"),
            ("nox_noun_atom(nox_noun_identity(input)[nox_noun_as_field(input)])","Error(TypeError)"),
        ] {
            let source=data::source(body);
            for bytes in [data::compile(&source),data::seed(&source)] {
                assert_eq!(data::run(&bytes,&data::nested()),Err(error.into()));
            }
        }
    });
}

fn fixture(name: &str) -> Vec<u8> {
    trident::compile_native_artifact_project(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/fixtures/{name}.tri")),
        &trident::CompileOptions::default(),
        trident::NativeArtifactProfile::RawNoun,
        trident::NATIVE_ARTIFACT_LIMITS,
    )
    .unwrap()
    .bytes
}
#[test]
fn digest_formula_depth_matches_the_independent_dag_for_shallow_and_deep_operands() {
    support::worker(|| {
        let probe = fixture("native_digest_depth");
        for expression in [
            "nox_noun_identity(nox_noun_atom(79))[2]",
            "nox_noun_identity(nox_noun_atom(sub(sub(sub(sub(sub(sub(sub(sub(115,1),2),3),4),5),6),7),8)))[2]",
            "nox_noun_identity(nox_noun_atom(79))[sub(sub(sub(sub(sub(sub(sub(sub(38,1),2),3),4),5),6),7),8)]",
        ] {
            let source=support::source(expression);
            let generate=|cap| codegen::generate_with_input::<{1<<20}>(&probe,|arena|{
                let source=support::data::Bytes::from_slice(arena,&source,4096).unwrap().encode(arena).unwrap();
                let cap=support::data::atom(arena,cap).unwrap();
                support::data::pair(arena,cap,source).unwrap()
            });
            let expected=generate(4096).unwrap();
            let exact=codegen::artifact_depth(&expected)+4;
            assert_eq!(generate(exact-1),Err(7));
            assert_eq!(generate(exact).unwrap(),expected);
            assert_eq!(generate(exact+1).unwrap(),expected);
            assert_eq!(support::run_artifact(&expected),words(&Atom(79))[2]);
        }
    });
}
#[test]
fn digest_index_nodes_and_delimiters_respect_separate_exact_capacity_limits() {
    support::worker(|| {
        let source = support::source("nox_noun_identity(nox_noun_atom(79))[2]");
        let mut prior = None;
        for cap in [4, 5, 6] {
            let mut caps = data::caps();
            caps[2] = 1;
            caps[3] = cap;
            match support::compile_only(&source, caps) {
                support::Compilation::Errors(errors) if cap == 4 => assert_eq!(errors[0].code, 7),
                support::Compilation::Program { bytes, .. } if cap >= 5 => {
                    assert_eq!(support::run_artifact(&bytes), words(&Atom(79))[2]);
                    if let Some(prior) = prior {
                        assert_eq!(bytes, prior);
                    }
                    prior = Some(bytes);
                }
                other => panic!("cap={cap}: {other:?}"),
            }
        }
        let probe = fixture("native_index_delimiters");
        for depth in [62, 63, 64] {
            let expected = if depth == 64 { 7064 } else { depth + 1 };
            assert_eq!(
                data::run(&probe, &Atom(depth)).unwrap().bytes,
                Atom(expected).encoded()
            );
        }
    });
}

#[test]
fn array_literals_and_annotations_follow_digest_bracket_support() {
    support::worker(|| {
        for body in [
            "fn main()->Field{let a=[1,2] 0}",
            "fn main()->Field{let a:[Field;2]=[1,2] 0}",
            "fn unused(a:[Field;2]){} fn main()->Field{0}",
            "fn unused()->[Field;2]{[1,2]} fn main()->Field{0}",
        ] {
            let source = format!("program sample {body}");
            match support::compile_only(source.as_bytes(), data::caps()) {
                support::Compilation::Program { bytes, .. } => {
                    assert_eq!(support::run_artifact(&bytes), 0, "{source}")
                }
                other => panic!("{source}: {other:?}"),
            }
        }
    });
}
