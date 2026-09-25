use super::support::{self, data, schema};
use nox::{artifact, sequential, NoTrace, Outcome, Reduction};
use std::sync::OnceLock;
use trident::NATIVE_ARTIFACT_LIMITS as LIMITS;

#[derive(Debug)]
struct Name {
    words: [usize; 9],
    prefix: String,
}

fn inspect(source: &str) -> Name {
    static FIXTURE: OnceLock<Vec<u8>> = OnceLock::new();
    let fixture = FIXTURE.get_or_init(|| {
        trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_qualified_name.tri"),
            &Default::default(),
            trident::NativeArtifactProfile::RawNoun,
            LIMITS,
        )
        .unwrap_or_else(|errors| panic!("fixture compilation: {errors:?}"))
        .bytes
    });
    let mut arena = Reduction::<{ 1 << 20 }>::try_new_boxed().unwrap();
    assert!(arena.limit_allocations(786432));
    let root = artifact::decode(&mut arena, fixture, LIMITS).unwrap();
    let mut cursor = arena.tail(root).unwrap();
    for _ in 0..3 {
        cursor = arena.tail(cursor).unwrap();
    }
    let formula = arena.head(cursor).unwrap();
    let input = schema::bytes(&mut arena, source.as_bytes()).unwrap();
    let execution = sequential::reduce(
        &mut arena,
        input,
        formula,
        100_000_000,
        sequential::Limits { max_frames: 65536 },
        &mut NoTrace,
    )
    .unwrap();
    let mut result = match execution.outcome {
        Outcome::Ok(result, _) => result,
        other => panic!("{other:?}; nodes={}", arena.count()),
    };
    let mut words = [0; 9];
    for word in &mut words {
        *word = arena
            .atom_value(arena.head(result).unwrap())
            .unwrap()
            .as_u64() as usize;
        result = arena.tail(result).unwrap();
    }
    let prefix = data::Bytes::decode(&mut arena, result, 255, 1000000).unwrap();
    let prefix = String::from_utf8(
        (0..prefix.len())
            .map(|i| prefix.get(&arena, i).unwrap())
            .collect(),
    )
    .unwrap();
    Name { words, prefix }
}

#[test]
fn qualified_names_normalize_only_prefix_and_preserve_original_member_spans() {
    support::worker(|| {
        let range = inspect("a..B");
        assert_eq!(range.words, [0, 0, 1, 0, 1, 0, 1, 33, 1]);
        assert!(range.prefix.is_empty());
        for (source, expected, member) in [
            ("X + 7".into(), "".into(), "X".into()),
            (
                "// ж😀\r\nstd // owner\n . config . VALUE (".into(),
                "std.config".into(),
                "VALUE".into(),
            ),
            (
                format!("{}.{} + 7", "a".repeat(255), "B".repeat(300)),
                "a".repeat(255),
                "B".repeat(300),
            ),
            (
                format!("{}.VALUE", "a.".repeat(127) + "b"),
                "a.".repeat(127) + "b",
                "VALUE".into(),
            ),
            ("A".repeat(4000), "".into(), "A".repeat(4000)),
        ] {
            let parsed = inspect(&source);
            assert_eq!(parsed.words[0], 0, "{parsed:?}");
            assert_eq!(parsed.prefix, expected);
            assert_eq!(&source[parsed.words[5]..parsed.words[6]], member);
            assert_eq!(parsed.words[4], parsed.words[6]);
            assert!(parsed.words[3] <= parsed.words[5]);
            assert!(parsed.words[8] >= parsed.words[6]);
        }
    });
}

#[test]
fn qualified_name_errors_preserve_lexical_syntax_and_prefix_capacity_boundaries() {
    support::worker(|| {
        for (source, code, offending) in [
            ("a.".into(), 2, "".into()),
            ("a. // missing\n fn".into(), 2, "fn".into()),
            ("a.$".into(), 1, "$".into()),
            ("a.B $".into(), 1, "$".into()),
            (format!("{}.B", "a".repeat(256)), 7, "a".repeat(256)),
            (format!("{}.c.B", "a".repeat(255)), 7, "c".into()),
            ("a. .B".into(), 2, ".".into()),
        ] {
            let parsed = inspect(&source);
            assert_eq!(parsed.words[0], code, "{parsed:?}");
            assert_eq!(&source[parsed.words[1]..parsed.words[2]], offending);
            if offending.is_empty() {
                assert_eq!(parsed.words[1], source.len());
            }
        }
    });
}
