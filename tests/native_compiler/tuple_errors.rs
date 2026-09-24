use super::{nouns::data, support};

#[test]
fn tuple_type_arity_mutability_and_recursive_noun_equality_fail_before_publication() {
    support::worker(|| {
        assert!(trident::compile_raw_artifact(
            &data::source("let(a,b)=(input,7) nox_noun_pair(a,nox_noun_atom(b))"),
            "oracle.tri",
            &trident::CompileOptions::default(),
            trident::NATIVE_ARTIFACT_LIMITS
        )
        .is_ok());
        for (body, code) in [
            ("let a:(Field)=7 input", 5),
            ("let a:(Field,Bool)=(7,9) input", 5),
            ("let a:(Field,(Field,Bool))=(7,(9,8)) input", 5),
            ("let(a)=(1,2) input", 5),
            ("let()=(1,2) input", 5),
            ("let(a,b)=7 input", 5),
            ("let(a,b,c)=nox_noun_identity(input) input", 5),
            ("let(a,b):(Field,Field)=(true,7) input", 5),
            ("let(a,b)=(7,9) (a,b)=(9,7) input", 5),
            ("let mut(a,b)=(7,9) (a,b)=(true,7) input", 5),
            ("let mut(a,b)=(7,9) (a,b)=(1,2,3) input", 5),
            ("let mut a=7 (a,missing)=(9,7) input", 5),
            ("let mut a=7 (a,2)=(9,7) input", 5),
            ("let mut a=7 ((a,a),a)=((9,7),2) input", 5),
            ("let a=(input,7) a==a input", 5),
            ("let a=((input,true),7) a==a input", 5),
            ("let a=(7,true) if a{return input} input", 5),
            ("let a=(7,9) a+a input", 5),
            ("let a=(7,9) a[0] input", 5),
            ("(1,) input", 2),
            ("() input", 2),
            ("(1,,2) input", 2),
            ("(1,2] input", 2),
            ("nox_noun_atom((1,2)) input", 5),
            ("let x:()=(1,2) input", 2),
            ("let x:(Field,)=(1,2) input", 2),
            ("let x:(Field,,Bool)=(1,true) input", 2),
        ] {
            let source = data::source(body);
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
    });
}

#[test]
fn tuple_arity_limits_and_hidden_named_discard_storage_have_exact_frontiers() {
    support::worker(|| {
        for (body, exact, expected) in [("let(a,b)=(7,9) a", 7, 7), ("let(_,_)=(7,9) 0", 5, 0)] {
            let source = support::source(body);
            let mut prior = None;
            for cap in [exact - 1, exact, exact + 1] {
                let mut caps = data::caps();
                caps[2] = 1;
                caps[3] = cap;
                match support::compile_only(&source, caps) {
                    support::Compilation::Errors(errors) if cap < exact => {
                        assert_eq!(errors[0].code, 7)
                    }
                    support::Compilation::Program { bytes, .. } if cap >= exact => {
                        assert_eq!(support::run_artifact(&bytes), expected);
                        if let Some(prior) = prior {
                            assert_eq!(bytes, prior);
                        }
                        prior = Some(bytes);
                    }
                    other => panic!("{body} cap={cap}: {other:?}"),
                }
            }
        }
        for count in [16, 17] {
            let value = vec!["7"; count].join(",");
            let names = (0..count)
                .map(|i| format!("x{i}"))
                .collect::<Vec<_>>()
                .join(",");
            let source = support::source(&format!("let({names})=({value}) x0"));
            match support::compile_only(&source, data::caps()) {
                support::Compilation::Program { bytes, .. } if count == 16 => {
                    assert_eq!(support::run_artifact(&bytes), 7)
                }
                support::Compilation::Errors(errors) if count == 17 => {
                    assert_eq!(errors[0].code, 7)
                }
                other => panic!("arity{count}: {other:?}"),
            }
        }
    });
}

#[test]
fn plain_groups_share_the_exact_operator_limit_with_call_and_index_delimiters() {
    support::worker(|| {
        let input = data::nested();
        for count in [63, 64] {
            for (body, expected) in [
                (
                    format!("nox_noun_atom({}7{})", "(".repeat(count), ")".repeat(count)),
                    7,
                ),
                (
                    format!(
                        "let d=nox_noun_identity(input) d[{}0{}] input",
                        "(".repeat(count),
                        ")".repeat(count)
                    ),
                    0,
                ),
            ] {
                let source = data::source(&body);
                match support::compile_only(source.as_bytes(), data::caps()) {
                    support::Compilation::Program { bytes, .. } if count == 63 => {
                        let result = data::run(&bytes, &input).unwrap();
                        let output = if body.starts_with("let") {
                            input.clone()
                        } else {
                            data::Noun::Atom(expected)
                        };
                        assert_eq!(result.bytes, output.encoded());
                    }
                    support::Compilation::Errors(errors) if count == 64 => {
                        assert_eq!(errors[0].code, 7)
                    }
                    other => panic!("groups={count} {body}: {other:?}"),
                }
            }
        }
    });
}
