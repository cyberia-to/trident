use super::{codegen, nouns::data, support};
use data::{Noun, Noun::Atom};

fn record(last_index: usize, last: Noun) -> Noun {
    let mut result = Atom(0);
    for index in (0..=last_index).rev() {
        let value = if index == last_index {
            last.clone()
        } else {
            Noun::pair(Atom(index as u64), Atom(100 + index as u64))
        };
        result = Noun::pair(value, result);
    }
    result
}

#[test]
fn persistent_edit_emission_preserves_all_siblings_across_axis_and_path_boundaries() {
    support::worker(|| {
        let probe = trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_record_edit.tri"),
            &trident::CompileOptions::default(),
            trident::NativeArtifactProfile::RawNoun,
            trident::NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap()
        .bytes;
        let mut paths = (61..=64)
            .map(|bits| vec![bits - 33, 31])
            .collect::<Vec<_>>();
        paths.push(vec![0, 31, 31]);
        paths.push(vec![0; 63]);
        paths.push(vec![0; 64]);
        for path in paths {
            let code = codegen::generate_with_input::<{ 1 << 20 }>(&probe, |arena| {
                let values = path
                    .iter()
                    .map(|index| support::data::atom(arena, *index as u64).unwrap())
                    .collect::<Vec<_>>();
                let seq = support::data::Seq::from_values(arena, &values, 64).unwrap();
                seq.encode(arena).unwrap()
            })
            .unwrap();
            let replacement = Noun::pair(Atom(99), Atom(999));
            let mut old = Atom(7);
            let mut new = replacement.clone();
            for index in path.iter().rev() {
                old = record(*index, old);
                new = record(*index, new);
            }
            let input = Noun::pair(old.clone(), replacement);
            let expected = Noun::pair(old, new);
            assert_eq!(
                data::run(&code, &input).unwrap().bytes,
                expected.encoded(),
                "path={path:?}"
            );
        }
    });
}
