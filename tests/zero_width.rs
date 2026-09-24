#[path = "native_control/support.rs"]
mod support;
use trident::{CompileOptions, RAW_ARTIFACT_LIMITS as LIMITS};

#[test]
fn raw_zero_width_values_preserve_neighbors_and_array_bounds() {
    support::worker(|| {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("zero_width.tri"),
            include_str!("fixtures/zero_width.tri"),
        )
        .unwrap();
        let path = directory.path().join("main.tri");
        for (function, expected) in [("keep", [17, 20]), ("read", [71, 71])] {
            std::fs::write(&path, format!("program zero\nuse zero_width\nfn main(input:Noun)->Noun {{ let n=nox_noun_as_field(input)\nnox_noun_atom(zero_width.{function}(n)) }}")).unwrap();
            for profile in ["debug", "release"] {
                let options = CompileOptions {
                    cfg_flags: std::collections::BTreeSet::from([profile.into()]),
                    ..CompileOptions::default()
                };
                let artifact =
                    trident::compile_raw_artifact_project(&path, &options, LIMITS).unwrap();
                for (input, expected) in expected.into_iter().enumerate() {
                    assert_eq!(
                        support::run(&artifact.bytes, input as u64, 1_000_000, 16384, 196608)
                            .unwrap()
                            .0,
                        expected,
                        "{function}/{profile}/{input}"
                    );
                }
                for input in [2, u32::MAX as u64] {
                    assert!(
                        support::run(&artifact.bytes, input, 1_000_000, 16384, 196608).is_err(),
                        "{function}/{profile}/{input}"
                    );
                }
            }
        }
    });
}
