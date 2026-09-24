use super::{codegen, signatures::run_component, support};

fn probe(name: &str) -> Vec<u8> {
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
fn graph_depth_counts_memoized_shared_suffixes_in_either_shortcut_order() {
    // Component only: the larger inline Reduction needs room for constructor
    // and return temporaries. Production JOBs still use their existing budget.
    std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(|| {
            let code = probe("native_call_graph_depth");
            for count in [127, 128, 129] {
                for order in [0, 1000] {
                    let (arena, result) =
                        run_component::<{ 1 << 20 }>("", count + order, &code, 786432);
                    let scalar = |order| arena.atom_value(order).unwrap().as_u64();
                    let error = scalar(arena.head(result).unwrap());
                    assert_eq!(error, if count <= 128 { 0 } else { 7 }, "{count}/{order}");
                    if error == 0 {
                        assert_eq!(scalar(arena.tail(result).unwrap()), count);
                    }
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn actual_function_table_and_call_frames_obey_exact_output_depth() {
    support::worker(|| {
        let code = probe("native_function_depth");
        let artifact = codegen::generate_from(4096, &code).unwrap();
        assert_eq!(support::run_artifact(&artifact), 12);
        // ART1 adds five levels, RES1 adds a further four.
        let required = codegen::artifact_depth(&artifact) + 4;
        assert_eq!(codegen::generate_from(required - 1, &code), Err(7));
        assert_eq!(codegen::generate_from(required, &code).unwrap(), artifact);
        assert_eq!(
            codegen::generate_from(required + 1, &code).unwrap(),
            artifact
        );
    });
}

#[test]
fn arguments_cross_chunk_and_balanced_frame_boundaries_in_source_order() {
    support::worker(|| {
        for count in [7, 8, 9] {
            let params = (0..count)
                .map(|i| format!("x{i}:Field"))
                .collect::<Vec<_>>()
                .join(",");
            let args = (1..=count)
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let source = format!(
                "program sample fn f({params})->Field{{x0*100+x{}}} fn main()->Field{{f({args})}}",
                count - 1
            );
            assert_eq!(
                support::value(support::compile(source.as_bytes())),
                100 + count
            );
            assert_eq!(support::rust_value(&source), 100 + count);
        }
    });
}

#[test]
fn actual_call_marker_push_checks_the_hard_delimiter_cap() {
    support::worker(|| {
        let code = probe("native_call_delimiters");
        assert_eq!(support::run_input(&code, 62), 63);
        assert_eq!(support::run_input(&code, 63), 64);
        assert_eq!(support::run_input(&code, 64), 7064);
    });
}
