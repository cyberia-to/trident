#[path = "native_control/support.rs"]
mod support;
use trident::{CompileOptions, RAW_ARTIFACT_LIMITS as LIMITS};

#[test]
fn raw_control_blocks_execute_constants_and_delimited_struct_expressions() {
    support::worker(|| {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("boundary.tri"),
            include_str!("fixtures/control_blocks.tri"),
        )
        .unwrap();
        let path = directory.path().join("main.tri");
        std::fs::write(&path,"program control_blocks\nuse boundary\nfn main(input:Noun)->Noun { let n=nox_noun_as_field(input)\nif n==boundary.ZERO {} else {}\nnox_noun_atom(boundary.choose(n)) }").unwrap();
        for profile in ["debug", "release"] {
            let options = CompileOptions {
                cfg_flags: std::collections::BTreeSet::from([profile.into()]),
                ..CompileOptions::default()
            };
            let artifact = trident::compile_raw_artifact_project(&path, &options, LIMITS).unwrap();
            for (input, expected) in [(0, 19), (1, 23)] {
                assert_eq!(
                    support::run(&artifact.bytes, input, 1_000_000, 16384, 196608)
                        .unwrap()
                        .0,
                    expected,
                    "{profile}/{input}"
                );
            }
        }
    });
}
