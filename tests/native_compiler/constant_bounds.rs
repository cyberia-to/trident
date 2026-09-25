use super::{nouns::data, signatures, support};

#[test]
fn native_constant_metadata_follows_final_visibility_and_original_literal_span() {
    support::worker(|| {
        let probe = trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_constants.tri"),
            &trident::CompileOptions::default(),
            trident::NativeArtifactProfile::RawNoun,
            trident::NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap()
        .bytes;
        for (decl, count, ty, public, literal, value) in [
            (
                "pub const A:U32=7 const A:Field=18446744069414584321",
                2,
                0,
                0,
                "18446744069414584321",
                0,
            ),
            (
                "const A:U32=7 pub const A:U32=((B)) const B:U32=0004294967295",
                3,
                3,
                1,
                "0004294967295",
                4294967295,
            ),
            (
                "const A:Field=B const B:Field=2 const B:Field=7",
                3,
                0,
                0,
                "7",
                7,
            ),
        ] {
            let source = format!("program sample {decl} fn main()->Field{{7}}");
            let (arena, result) = signatures::run_probe(&source, 4096, &probe);
            let mut cursor = result;
            for expected in [0, count, ty, public, value] {
                assert_eq!(
                    arena
                        .atom_value(arena.head(cursor).unwrap())
                        .unwrap()
                        .as_u64(),
                    expected,
                    "{source}"
                );
                cursor = arena.tail(cursor).unwrap();
            }
            let start = arena
                .atom_value(arena.head(cursor).unwrap())
                .unwrap()
                .as_u64() as usize;
            let end = arena
                .atom_value(arena.tail(cursor).unwrap())
                .unwrap()
                .as_u64() as usize;
            assert_eq!(&source[start..end], literal);
        }
    });
}

#[test]
fn native_constant_declarations_grouping_and_expression_nodes_keep_independent_caps() {
    support::worker(|| {
        let source =
            "program sample const A:Field=B const B:Field=C const C:Field=7 fn main()->Field{A}";
        let mut previous = None;
        for cap in [2, 3, 4] {
            let mut caps = data::caps();
            caps[2] = 1;
            caps[3] = cap;
            match support::compile_only(source.as_bytes(), caps) {
                support::Compilation::Errors(errors) if cap == 2 => assert_eq!(errors[0].code, 7),
                support::Compilation::Program { bytes, .. } if cap >= 3 => {
                    assert_eq!(support::run_artifact(&bytes), 7);
                    if let Some(ref previous) = previous {
                        assert_eq!(&bytes, previous)
                    }
                    previous = Some(bytes);
                }
                other => panic!("cap={cap}: {other:?}"),
            }
        }
        for (groups, cap, success) in [
            (2, 1, false),
            (2, 2, true),
            (63, 4096, true),
            (64, 4096, true),
            (65, 4096, false),
        ] {
            let source = format!(
                "program sample const A:Field={}7{} fn main()->Field{{A}}",
                "(".repeat(groups),
                ")".repeat(groups)
            );
            let mut caps = data::caps();
            caps[2] = 1;
            caps[3] = cap;
            match support::compile_only(source.as_bytes(), caps) {
                support::Compilation::Errors(errors) if !success => {
                    assert_eq!(errors[0].code, 7);
                    assert_eq!(
                        &source[errors[0].start as usize..errors[0].end as usize],
                        "("
                    );
                }
                support::Compilation::Program { bytes, .. } if success => {
                    assert_eq!(support::run_artifact(&bytes), 7)
                }
                other => panic!("groups={groups} cap={cap}: {other:?}"),
            }
        }
        // One declaration does not consume one expression node. A+A has three.
        let source = b"program sample const A:Field=7 fn main()->Field{A+A}";
        for cap in [2, 3] {
            let mut caps = data::caps();
            caps[2] = 1;
            caps[3] = cap;
            match support::compile_only(source, caps) {
                support::Compilation::Errors(errors) if cap == 2 => assert_eq!(errors[0].code, 7),
                support::Compilation::Program { bytes, .. } if cap == 3 => {
                    assert_eq!(support::run_artifact(&bytes), 14)
                }
                other => panic!("cap={cap}: {other:?}"),
            }
        }
    });
}

#[test]
fn native_constant_alias_chain_uses_memoized_iteration_and_shared_literal_identity() {
    support::worker(|| {
        for count in [1, 8, 16] {
            let declarations = (0..count)
                .map(|i| format!("const C{i}:Field=C{} ", i + 1))
                .collect::<String>();
            let source=format!("program sample {declarations} const C{count}:Field=7 fn main()->Field{{C0+C{count}}}");
            let code = data::compile(&source);
            assert_eq!(support::run_artifact(&code), 14);
            assert_eq!(code, data::compile("program sample fn main()->Field{7+7}"));
        }
    });
}
