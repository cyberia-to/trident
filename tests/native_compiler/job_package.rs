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
                .join("tests/fixtures/native_job_package.tri"),
            &Default::default(),
            trident::NativeArtifactProfile::RawNoun,
            LIMITS,
        )
        .unwrap()
        .bytes
    })
}

fn module(path: &str, source: &[u8]) -> schema::Module {
    schema::Module {
        path: path.into(),
        source: source.into(),
        origin: "fixture".into(),
        version: "1".into(),
    }
}

#[derive(Debug)]
struct Observation {
    entry: u64,
    found: u64,
    index: u64,
    admitted: u64,
    searched: u64,
    opened: u64,
    code: u64,
    content: Vec<u8>,
    nodes: u32,
    frames: u32,
    reductions: u64,
}

fn run(
    modules: &[schema::Module],
    name: &str,
    capacity: u64,
    remaining: u64,
) -> Result<Observation, String> {
    let mut arena = Arena::try_new_boxed().unwrap();
    assert!(arena.limit_allocations(786432));
    let program = artifact::decode(&mut arena, fixture(), LIMITS).unwrap();
    let mut cursor = arena.tail(program).unwrap();
    for _ in 0..3 {
        cursor = arena.tail(cursor).unwrap();
    }
    let formula = arena.head(cursor).unwrap();
    let mut caps = support::CAPS;
    caps[0] = 8192;
    caps[1] = 65536;
    caps[3] = 65536;
    caps[9] = 786432;
    let options = schema::Options {
        cfg: vec!["explicit".into()],
        ..support::options()
    };
    let job = schema::job(
        &mut arena, program, modules, "entry", "main", &options, &caps,
    )
    .unwrap();
    let mut opts = arena.tail(job).unwrap();
    for _ in 0..4 {
        opts = arena.tail(opts).unwrap();
    }
    let opts = arena.head(opts).unwrap();
    let query = schema::bytes(&mut arena, name.as_bytes()).unwrap();
    let cap = schema::atom(&mut arena, capacity).unwrap();
    let remaining = schema::atom(&mut arena, remaining).unwrap();
    let limits = schema::pair(&mut arena, cap, remaining).unwrap();
    let arguments = schema::pair(&mut arena, query, limits).unwrap();
    let input = schema::pair(&mut arena, job, arguments).unwrap();
    let execution = sequential::reduce(
        &mut arena,
        input,
        formula,
        100_000_000,
        sequential::Limits { max_frames: 65536 },
        &mut NoTrace,
    )
    .map_err(|error| format!("{error:?}"))?;
    let (mut result, reductions) = match execution.outcome {
        Outcome::Ok(value, remaining) => (value, 100_000_000 - remaining),
        other => {
            return Err(format!(
                "{other:?}; nodes={}; frames={}",
                arena.count(),
                execution.peak_frames
            ))
        }
    };
    let mut words = Vec::new();
    for _ in 0..7 {
        words.push(
            arena
                .atom_value(arena.head(result).unwrap())
                .unwrap()
                .as_u64(),
        );
        result = arena.tail(result).unwrap();
    }
    assert_eq!(
        arena.tail(result).unwrap(),
        opts,
        "explicit OPT1 must survive entry admission"
    );
    let content = arena.head(result).unwrap();
    let decoded = data::Bytes::decode(&mut arena, content, 4096, 1000000).unwrap();
    let content = (0..decoded.len())
        .map(|i| decoded.get(&arena, i).unwrap())
        .collect();
    Ok(Observation {
        entry: words[0],
        found: words[1],
        index: words[2],
        admitted: words[3],
        searched: words[4],
        opened: words[5],
        code: words[6],
        content,
        nodes: arena.count(),
        frames: execution.peak_frames,
        reductions,
    })
}

#[test]
fn package_lookup_and_source_open_share_the_remaining_allowance() {
    support::worker(|| {
        let modules = [
            module("a", b"unused"),
            module("entry", b"program entry fn main()->Field{7}"),
            module("z.value", b"module z.value pub const VALUE:Field=9"),
        ];
        let observed = run(&modules, "z.value", 4096, 0).unwrap();
        assert_eq!(
            (
                observed.entry,
                observed.found,
                observed.index,
                observed.code
            ),
            (1, 1, 2, 0)
        );
        assert_eq!(observed.content, modules[2].source);
        assert!(observed.admitted < support::CAPS[4]);
        // Two probes: path3 + MOD1 record8 + name slot1 each. The unequal
        // entry path adds three tree visits, two two-level word reads and
        // two two-level byte reads for the first differing byte.
        assert_eq!(
            observed.admitted - observed.searched,
            2 * (3 + 8 + 1) + 3 + 2 * 2 + 2 * 2
        );
        assert!(observed.opened < observed.searched);
        let required = observed.admitted - observed.opened;
        assert_eq!(run(&modules, "z.value", 4096, required).unwrap().opened, 0);
        assert!(run(&modules, "z.value", 4096, required - 1).is_err());
        let rejected = run(&modules, "z.value", 1, 0).unwrap();
        assert_eq!(rejected.code, 7);
        assert!(rejected.content.is_empty());
        assert_eq!(rejected.searched - rejected.opened, 3 + 8 + 7);
        let missing = run(&modules, "z.absent", 4096, 0).unwrap();
        assert_eq!((missing.found, missing.index), (0, 3));
        assert_eq!(missing.opened, missing.searched);
    });
}

#[test]
fn package_lookup_distinguishes_prefixes_and_the_last_allowed_name_byte() {
    support::worker(|| {
        let prefix = "x".repeat(254);
        let last_a = format!("{prefix}a");
        let last_b = format!("{prefix}b");
        let last_c = format!("{prefix}c");
        let modules = [
            module("a", b"first"),
            module("a.b", b"child"),
            module("entry", b"program entry fn main()->Field{7}"),
            module(&last_a, b"long a"),
            module(&last_c, b"long c"),
        ];
        for (index, item) in modules.iter().enumerate() {
            let observed = run(&modules, &item.path, 4096, 0).unwrap();
            assert_eq!((observed.found, observed.index), (1, index as u64));
            assert_eq!(observed.content, item.source);
        }
        for name in ["A", "a.a", "a.b.c", "z", &last_b] {
            let observed = run(&modules, name, 4096, 0).unwrap();
            assert_eq!((observed.found, observed.index), (0, modules.len() as u64));
        }
        // A missing long-prefix query spends its final visits comparing bytes,
        // with no subsequent source-open phase to mask a comparison undercharge.
        let observed = run(&modules, &last_b, 4096, 0).unwrap();
        let required = observed.admitted - observed.searched;
        let exact = run(&modules, &last_b, 4096, required).unwrap();
        assert_eq!((exact.found, exact.searched), (0, 0));
        assert!(run(&modules, &last_b, 4096, required - 1).is_err());
        assert!(run(&modules, &"x".repeat(256), 4096, 0).is_err());
    });
}

#[test]
fn package_indices_remain_distinct_from_the_source_capacity_sentinel() {
    support::worker(|| {
        let mut modules = vec![module("entry", b"program entry fn main()->Field{7}")];
        for index in 0..4096 {
            modules.push(module(&format!("z{index:04}"), &[0xff]));
        }
        let admitted = run(&modules, "z4095", 1, u32::MAX.into()).expect("entry admission");
        assert_eq!(admitted.entry, 0);
        // Source bytes are admitted as bytes; the resolver owns reachable UTF-8.
        let found = run(&modules, "z4095", 1, 0).unwrap();
        assert_eq!((found.found, found.index, found.code), (1, 4096, 0));
        assert_eq!(found.content, [0xff]);
        let missing = run(&modules, "z4096", 1, 0).unwrap();
        assert_eq!((missing.found, missing.index), (0, 4097));
        for (case, observed) in [
            ("admission", admitted),
            ("found", found),
            ("missing", missing),
        ] {
            assert!(observed.nodes < 786432);
            assert!(observed.frames < 65536);
            println!(
                "package4097 {case}: reductions={}, nodes={}, frames={}",
                observed.reductions, observed.nodes, observed.frames
            );
        }
    });
}
