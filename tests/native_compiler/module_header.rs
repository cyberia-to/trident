use super::support::{self, data, schema};
use nox::{artifact, sequential, NoTrace, Outcome, Reduction};
use std::sync::OnceLock;
use trident::NATIVE_ARTIFACT_LIMITS as LIMITS;

type Arena = Reduction<{ 1 << 20 }>;
fn fixture() -> &'static [u8] {
    static FIXTURE: OnceLock<Vec<u8>> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_module_header.tri"),
            &Default::default(),
            trident::NativeArtifactProfile::RawNoun,
            LIMITS,
        )
        .unwrap_or_else(|errors| panic!("fixture compilation: {:?}", errors.first()))
        .bytes
    })
}

#[derive(Debug)]
struct Import {
    name: String,
    start: usize,
    path_start: usize,
    end: usize,
}
#[derive(Debug)]
struct Header {
    code: u64,
    start: usize,
    end: usize,
    body_kind: u64,
    body_start: usize,
    imports: Vec<Import>,
}

fn read_word(arena: &Arena, value: nox::Order) -> u64 {
    arena.atom_value(value).unwrap().as_u64()
}

fn inspect(source: &str, owner: &str, program: bool) -> Header {
    let mut arena = Arena::try_new_boxed().unwrap();
    assert!(arena.limit_allocations(786432));
    let root = artifact::decode(&mut arena, fixture(), LIMITS).unwrap();
    let mut cursor = arena.tail(root).unwrap();
    for _ in 0..3 {
        cursor = arena.tail(cursor).unwrap();
    }
    let formula = arena.head(cursor).unwrap();
    let content = schema::bytes(&mut arena, source.as_bytes()).unwrap();
    let name = schema::bytes(&mut arena, owner.as_bytes()).unwrap();
    let kind = schema::atom(&mut arena, u64::from(program)).unwrap();
    let tail = schema::pair(&mut arena, name, kind).unwrap();
    let input = schema::pair(&mut arena, content, tail).unwrap();
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
        other => panic!(
            "{other:?}; nodes={}; frames={}",
            arena.count(),
            execution.peak_frames
        ),
    };
    let mut words = Vec::new();
    for _ in 0..5 {
        words.push(read_word(&arena, arena.head(result).unwrap()));
        result = arena.tail(result).unwrap();
    }
    let values = data::Seq::decode(&mut arena, result, 4096, 1000000).unwrap();
    let mut imports = Vec::new();
    for index in 0..values.len() {
        let item = values.get(&arena, index).unwrap();
        let name = arena.head(item).unwrap();
        let bytes = data::Bytes::decode(&mut arena, name, 255, 1000000).unwrap();
        let name = String::from_utf8(
            (0..bytes.len())
                .map(|i| bytes.get(&arena, i).unwrap())
                .collect(),
        )
        .unwrap();
        let span = arena.tail(item).unwrap();
        let path = arena.tail(span).unwrap();
        imports.push(Import {
            name,
            start: read_word(&arena, arena.head(span).unwrap()) as usize,
            path_start: read_word(&arena, arena.head(path).unwrap()) as usize,
            end: read_word(&arena, arena.tail(path).unwrap()) as usize,
        });
    }
    Header {
        code: words[0],
        start: words[1] as usize,
        end: words[2] as usize,
        body_kind: words[3],
        body_start: words[4] as usize,
        imports,
    }
}

#[test]
fn native_module_headers_preserve_repeated_uses_and_original_byte_spans() {
    support::worker(|| {
        let first = "use alpha // a use inside a comment\n . value";
        let source = format!("// ж😀\r\nmodule pkg // owner\n . unit\t{first}\nuse beta.value\nuse alpha.value\nfn body(){{%}}");
        let parsed = inspect(&source, "pkg.unit", false);
        assert_eq!(parsed.code, 0, "{parsed:?}");
        assert_eq!(parsed.body_kind, 11);
        assert_eq!(parsed.body_start, source.find("fn body").unwrap());
        assert_eq!(
            parsed
                .imports
                .iter()
                .map(|i| i.name.as_str())
                .collect::<Vec<_>>(),
            ["alpha.value", "beta.value", "alpha.value"]
        );
        let one = &parsed.imports[0];
        assert_eq!(&source[one.start..one.end], first);
        assert_eq!(&source[one.path_start..one.end], &first[4..]);
        for item in &parsed.imports {
            assert!(
                item.start < item.path_start
                    && item.path_start < item.end
                    && item.end <= source.len()
            );
        }
        let entry = inspect("program entry use alpha fn main()->Field{7}", "entry", true);
        assert_eq!(entry.code, 0);
        assert_eq!(entry.imports[0].name, "alpha");
    });
}

#[test]
fn native_module_headers_reject_kind_owner_and_incomplete_paths_precisely() {
    support::worker(|| {
        for (source, owner, program, code, offending) in [
            ("program entry", "entry", false, 3, "program"),
            ("module entry", "entry", true, 3, "module"),
            ("module a // name\n . b", "a.c", false, 3, "a // name\n . b"),
            ("program entry.other", "entry", true, 2, "."),
            ("program entry use a.", "entry", true, 2, ""),
            ("program entry use a. fn main(){}", "entry", true, 2, "fn"),
            ("module pkg. use a", "pkg", false, 2, "use"),
            ("program entry use a.$", "entry", true, 1, "$"),
            ("program entry $", "entry", true, 1, "$"),
            ("program entry use a $", "entry", true, 1, "$"),
        ] {
            let parsed = inspect(source, owner, program);
            assert_eq!(parsed.code, code, "{source}: {parsed:?}");
            assert_eq!(&source[parsed.start..parsed.end], offending, "{source}");
            if offending.is_empty() {
                assert_eq!(parsed.start, source.len());
            }
        }
    });
}

#[test]
fn native_path_capacity_counts_normalized_bytes_without_truncating_names() {
    support::worker(|| {
        let owner = "a".repeat(255);
        assert_eq!(inspect(&format!("module {owner}"), &owner, false).code, 0);
        let exact = format!("{}b", "a.".repeat(127));
        let spaced = exact.replace('.', " // part\n . ");
        let source = format!("program entry use {spaced}");
        assert!(source.len() < 4096);
        let parsed = inspect(&source, "entry", true);
        assert_eq!(parsed.code, 0, "{parsed:?}");
        assert_eq!(parsed.imports[0].name, exact);
        for path in ["a".repeat(256), format!("{exact}.b")] {
            let parsed = inspect(&format!("program entry use {path}"), "entry", true);
            assert_eq!(parsed.code, 7, "{parsed:?}");
        }
        let wrong = format!("{}c", "a.".repeat(127));
        assert_eq!(inspect(&format!("module {exact}"), &wrong, false).code, 3);
    });
}
