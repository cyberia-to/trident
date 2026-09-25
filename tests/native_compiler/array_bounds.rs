use super::{codegen, nouns::data, support, type_syntax, types::tuple};
use data::Noun::{self, Atom};

#[test]
fn array_types_count_logical_elements_and_preserve_canonical_identity() {
    support::worker(|| {
        let array = |n| {
            Noun::pair(
                Atom(18),
                Noun::pair(
                    Noun::pair(Atom(0), Atom(n)),
                    Noun::pair(Atom(1), Noun::pair(Atom(n + 1), Atom(0))),
                ),
            )
        };
        for (text, ty, cap) in [
            ("[Field;0]", array(0), 1),
            ("[Field;33]", array(33), 34),
            ("[Field;00033]", array(33), 34),
            ("([Field;2],Field)", tuple(&[array(2), Atom(0)], 2, 5, 0), 5),
        ] {
            let (error, actual) = type_syntax::parse(text, cap);
            assert_eq!(error, 0, "{text}");
            let end = text.len() as u64;
            assert_eq!(
                actual,
                Noun::pair(Noun::pair(Atom(0), Noun::pair(Atom(end), Atom(end))), ty).encoded(),
                "{text}"
            );
            assert_eq!(type_syntax::parse(text, cap - 1).0, 7, "{text}");
        }
        for n in [
            4095u64,
            4096,
            4294967296,
            18446744069414584321,
            18446744073709551615,
        ] {
            assert_eq!(
                type_syntax::parse(&format!("[Field;{n}]"), 4096).0,
                if n < 4096 { 0 } else { 7 }
            );
        }
        for n in [62, 63, 64] {
            let text = format!("{}[Field;0]{}", "(".repeat(n), ")".repeat(n));
            assert_eq!(
                type_syntax::parse(&text, 4096).0,
                if n < 64 { 0 } else { 7 }
            );
        }
    });
}

#[test]
fn array_helper_admission_counts_one_slot_after_all_functions_and_loops() {
    support::worker(|| {
        let probe = trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_array_plan.tri"),
            &trident::CompileOptions::default(),
            trident::NativeArtifactProfile::RawNoun,
            trident::NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap()
        .bytes;
        for (functions, loops, needed, cap, failed, total, slot) in [
            (1, 0, 0, 1, 0, 1, 4096),
            (1, 0, 1, 1, 1, 1, 4096),
            (1, 0, 1, 2, 0, 2, 1),
            (4, 2, 1, 12, 1, 12, 4096),
            (4, 2, 1, 13, 0, 13, 12),
            (4096, 0, 0, 4096, 0, 4096, 4096),
            (4096, 0, 1, 4096, 1, 4096, 4096),
        ] {
            let mut order = (0..functions).map(Atom).collect::<Vec<_>>();
            while order.len() > 1 {
                order = order
                    .chunks_exact(2)
                    .map(|pair| Noun::pair(pair[0].clone(), pair[1].clone()))
                    .collect();
            }
            let ids = Noun::pair(
                Atom(1397051697),
                Noun::pair(Atom(functions), order.pop().unwrap()),
            );
            let input = Noun::pair(
                ids,
                Noun::pair(Atom(loops), Noun::pair(Atom(needed), Atom(cap))),
            );
            // The 4096-body component constructs an artificial full table,
            // beyond current source/graph acceptance. Keep its budget explicit;
            // this does not raise the whole compiler JOB arena allowance.
            let nodes = if functions == 4096 { 12582912 } else { 196608 };
            let result = data::run_traced(
                &probe,
                &input,
                1_000_000_000,
                65536,
                nodes,
                &mut nox::NoTrace,
            )
            .unwrap_or_else(|e| panic!("functions={functions} loops={loops} needed={needed}: {e}"));
            eprintln!("table component: functions={functions} needed={needed} reductions={} nodes={} frames={}",result.reductions,result.nodes,result.frames);
            assert_eq!(
                result.bytes,
                Noun::pair(Atom(failed), Noun::pair(Atom(total), Atom(slot))).encoded(),
                "f={functions},l={loops},g={needed},cap={cap}"
            );
        }
    });
}

#[test]
fn array_reads_and_values_report_exact_complete_artifact_depth() {
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
        let values = (0..33).map(|i| i.to_string()).collect::<Vec<_>>().join(",");
        for source in [
            "program sample fn main()->Field{[7][0]}".to_string(),
            format!("program sample fn main()->Field{{let a=[{values}] 7}}"),
            "program sample fn main()->Field{[sub(sub(sub(sub(sub(sub(sub(sub(43,1),2),3),4),5),6),7),8)][0]}".to_string(),
            "program sample fn main()->Field{[7][sub(sub(sub(sub(sub(sub(sub(sub(36,1),2),3),4),5),6),7),8)]}".to_string(),
            "program sample fn f(a:[Field;2])->Field{a[1]} fn main()->Field{f([7,9])}".to_string(),
        ] {
            let generate=|cap|codegen::generate_with_input::<{1<<20}>(&probe,|arena|{
                let source=support::data::Bytes::from_slice(arena,source.as_bytes(),4096).unwrap().encode(arena).unwrap();
                let cap=support::data::atom(arena,cap).unwrap();support::data::pair(arena,cap,source).unwrap()
            });
            let code=generate(4096).unwrap();let exact=codegen::artifact_depth(&code)+4;
            assert_eq!(generate(exact-1),Err(7),"{source}");assert_eq!(generate(exact).unwrap(),code,"{source}");assert_eq!(generate(exact+1).unwrap(),code);
        }
    });
}

#[test]
fn array_markers_and_expression_nodes_keep_independent_exact_allowances() {
    support::worker(|| {
        let probe = trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_array_delimiters.tri"),
            &trident::CompileOptions::default(),
            trident::NativeArtifactProfile::RawNoun,
            trident::NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap()
        .bytes;
        for (depth, result) in [(63, 64), (64, 7064)] {
            assert_eq!(
                data::run(&probe, &Atom(depth)).unwrap().bytes,
                Atom(result).encoded()
            );
        }
        let source = "program sample fn main()->Field{[7][0]}";
        let mut previous = None;
        for cap in [2, 3, 4, 5] {
            let mut caps = data::caps();
            caps[2] = 1;
            caps[3] = cap;
            match support::compile_only(source.as_bytes(), caps) {
                support::Compilation::Errors(errors) if cap < 4 => assert_eq!(errors[0].code, 7),
                support::Compilation::Program { bytes, .. } if cap >= 4 => {
                    assert_eq!(support::run_artifact(&bytes), 7);
                    if let Some(ref previous) = previous {
                        assert_eq!(&bytes, previous);
                    }
                    previous = Some(bytes);
                }
                other => panic!("cap={cap}: {other:?}"),
            }
        }
    });
}
