use super::support::{self, data, schema, validate};
use nox::{artifact, sequential, NoTrace, Order, Outcome, Reduction};
use std::sync::OnceLock;
use trident::NATIVE_ARTIFACT_LIMITS as LIMITS;

type Arena = Reduction<{ 1 << 20 }>;
fn fixture() -> &'static [u8] {
    static FIXTURE: OnceLock<Vec<u8>> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_module_graph.tri"),
            &Default::default(),
            trident::NativeArtifactProfile::RawNoun,
            LIMITS,
        )
        .unwrap_or_else(|errors| panic!("fixture compilation: {:?}", errors.first()))
        .bytes
    })
}

fn module(path: &str, source: impl AsRef<[u8]>) -> schema::Module {
    schema::Module {
        path: path.into(),
        source: source.as_ref().into(),
        origin: "graph".into(),
        version: "1".into(),
    }
}

#[derive(Debug)]
struct Use {
    target: usize,
    start: usize,
    path_start: usize,
    end: usize,
}
#[derive(Debug)]
struct Module {
    index: usize,
    name: String,
    source: Vec<u8>,
    offset: usize,
    uses: Vec<Use>,
}
#[derive(Debug)]
struct Graph {
    index: usize,
    code: u64,
    start: usize,
    end: usize,
    admitted: u64,
    remaining: u64,
    bytes: usize,
    uses: usize,
    modules: Vec<Module>,
    order: Vec<usize>,
}

fn word(arena: &Arena, value: Order) -> u64 {
    arena.atom_value(value).unwrap().as_u64()
}
fn next(arena: &Arena, cursor: &mut Order) -> Order {
    let value = arena.head(*cursor).unwrap();
    *cursor = arena.tail(*cursor).unwrap();
    value
}
fn scalar(arena: &Arena, cursor: &mut Order) -> u64 {
    word(arena, next(arena, cursor))
}
fn bytes(arena: &mut Arena, value: Order) -> Vec<u8> {
    let value = data::Bytes::decode(arena, value, 4096, 1000000).unwrap();
    (0..value.len())
        .map(|i| value.get(arena, i).unwrap())
        .collect()
}

fn admitted_job(modules: &[schema::Module], cap: u64) -> Vec<u8> {
    let mut arena = Arena::try_new_boxed().unwrap();
    assert!(arena.limit_allocations(786432));
    let mut caps = support::CAPS;
    // Host source allowance exceeds the unchanged4096 guest closure cap.
    caps[0] = 8192;
    caps[1] = 65536;
    caps[3] = cap;
    caps[9] = 786432;
    let options = schema::Options {
        cfg: vec!["explicit".into()],
        ..support::options()
    };
    let compiler = artifact::decode(&mut arena, support::compiler(), LIMITS).unwrap();
    let job = schema::job(
        &mut arena, compiler, modules, "entry", "main", &options, &caps,
    )
    .unwrap();
    // Independent admission includes the real compiler. Only the resulting
    // JOB1 enters the separate raw component executor below.
    validate::job(&mut arena, job, compiler, caps).unwrap();
    artifact::encode(&arena, job, LIMITS).unwrap()
}

fn run(modules: &[schema::Module], cap: u64, allowance: u64) -> Result<Graph, String> {
    let job_bytes = admitted_job(modules, cap);
    let mut arena = Arena::try_new_boxed().unwrap();
    assert!(arena.limit_allocations(786432));
    let root = artifact::decode(&mut arena, fixture(), LIMITS).unwrap();
    let mut cursor = arena.tail(root).unwrap();
    for _ in 0..3 {
        cursor = arena.tail(cursor).unwrap();
    }
    let formula = arena.head(cursor).unwrap();
    let job = artifact::decode(&mut arena, &job_bytes, LIMITS).unwrap();
    let mut opts = arena.tail(job).unwrap();
    for _ in 0..4 {
        opts = arena.tail(opts).unwrap();
    }
    let opts = arena.head(opts).unwrap();
    let limit = schema::atom(&mut arena, allowance).unwrap();
    let input = schema::pair(&mut arena, job, limit).unwrap();
    let execution = sequential::reduce(
        &mut arena,
        input,
        formula,
        100_000_000,
        sequential::Limits { max_frames: 65536 },
        &mut NoTrace,
    )
    .map_err(|e| format!("{e:?}"))?;
    let (mut result, remaining) = match execution.outcome {
        Outcome::Ok(result, remaining) => (result, remaining),
        other => {
            return Err(format!(
                "{other:?}; nodes={}; frames={}",
                arena.count(),
                execution.peak_frames
            ))
        }
    };
    if modules.len() > 4096 {
        eprintln!(
            "graph4097: reductions={}, nodes={}, frames={}",
            100_000_000 - remaining,
            arena.count(),
            execution.peak_frames
        );
    }
    let mut values = Vec::new();
    for _ in 0..8 {
        values.push(scalar(&arena, &mut result));
    }
    let raw = next(&arena, &mut result);
    let decoded = data::Seq::decode(&mut arena, raw, 4096, 1000000).unwrap();
    let mut modules = Vec::new();
    for id in 0..decoded.len() {
        let mut item = decoded.get(&arena, id).unwrap();
        let index = scalar(&arena, &mut item) as usize;
        let raw = next(&arena, &mut item);
        let name = String::from_utf8(bytes(&mut arena, raw)).unwrap();
        let raw = next(&arena, &mut item);
        let source = bytes(&mut arena, raw);
        let offset = scalar(&arena, &mut item) as usize;
        let status = scalar(&arena, &mut item);
        let count = scalar(&arena, &mut item);
        if values[1] == 0 {
            assert_eq!(status, 2);
        }
        let mut uses = Vec::new();
        for _ in 0..count {
            let mut occurrence = next(&arena, &mut item);
            uses.push(Use {
                target: scalar(&arena, &mut occurrence) as usize,
                start: scalar(&arena, &mut occurrence) as usize,
                path_start: scalar(&arena, &mut occurrence) as usize,
                end: word(&arena, occurrence) as usize,
            });
        }
        assert_eq!(word(&arena, item), 0);
        modules.push(Module {
            index,
            name,
            source,
            offset,
            uses,
        });
    }
    let raw = next(&arena, &mut result);
    assert_eq!(
        result, opts,
        "OPT1 must remain exact through graph discovery"
    );
    let order = data::Seq::decode(&mut arena, raw, 4096, 1000000).unwrap();
    let order = (0..order.len())
        .map(|i| word(&arena, order.get(&arena, i).unwrap()) as usize)
        .collect();
    Ok(Graph {
        index: values[0] as usize,
        code: values[1],
        start: values[2] as usize,
        end: values[3] as usize,
        admitted: values[4],
        remaining: values[5],
        bytes: values[6] as usize,
        uses: values[7] as usize,
        modules,
        order,
    })
}

#[test]
fn native_module_graph_preserves_occurrences_shares_diamonds_and_matches_seed_order() {
    support::worker(|| {
        // Entry DFS yields z,b,a,entry; the seed starts from lexical root a.
        let modules = [
            module("a", "module a use z"),
            module("b", "module b use z"),
            module(
                "entry",
                "program entry use b use a use b fn main()->Field{7}",
            ),
            module("unused", [255]),
            module("z", "module z"),
        ];
        let graph = run(&modules, 4096, 0).unwrap();
        assert_eq!(graph.code, 0, "{graph:?}");
        assert_eq!(
            graph
                .modules
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<_>>(),
            ["entry", "b", "z", "a"]
        );
        assert_eq!(
            graph
                .order
                .iter()
                .map(|&i| graph.modules[i].name.as_str())
                .collect::<Vec<_>>(),
            ["z", "a", "b", "entry"]
        );
        assert_eq!(graph.uses, 5);
        assert_eq!(
            graph.bytes,
            modules
                .iter()
                .filter(|m| m.path != "unused")
                .map(|m| m.source.len())
                .sum::<usize>()
        );
        let entry = &graph.modules[0];
        assert_eq!(
            entry.uses.iter().map(|u| u.target).collect::<Vec<_>>(),
            [1, 3, 1]
        );
        for item in &graph.modules {
            assert_eq!(item.source, modules[item.index].source);
            assert_eq!(
                &item.source[item.offset..],
                if item.name == "entry" {
                    b"fn main()->Field{7}".as_slice()
                } else {
                    b""
                }
            );
            for used in &item.uses {
                assert_eq!(&item.source[used.start..used.path_start], b"use ");
                assert_eq!(
                    &item.source[used.path_start..used.end],
                    graph.modules[used.target].name.as_bytes()
                );
            }
        }
        assert!(graph.remaining < graph.admitted);
        let independent = [
            module("a", "module a use z"),
            module("b", "module b"),
            module("entry", "program entry use b use a"),
            module("z", "module z"),
        ];
        let graph = run(&independent, 4096, 0).unwrap();
        assert_eq!(graph.code, 0);
        // Lexical-ready Kahn would emit b,z,a,entry, unlike the seed's DFS.
        assert_eq!(
            graph
                .order
                .iter()
                .map(|&i| graph.modules[i].name.as_str())
                .collect::<Vec<_>>(),
            ["z", "a", "b", "entry"]
        );
    });
}

#[test]
fn native_module_graph_keeps_package_index4096_distinct_from_compact_ids() {
    support::worker(|| {
        let mut modules = vec![module("entry", "program entry use z")];
        modules.extend((0..4095).map(|i| module(&format!("m{i:04}"), [255])));
        modules.push(module("z", "module z"));
        let graph = run(&modules, 65536, 0).unwrap();
        assert_eq!(graph.code, 0, "{graph:?}");
        assert_eq!(
            graph.modules.iter().map(|m| m.index).collect::<Vec<_>>(),
            [0, 4096]
        );
        assert_eq!(graph.modules[0].uses[0].target, 1);
        assert_eq!(graph.order, [1, 0]);
        assert_eq!(
            graph.bytes,
            modules[0].source.len() + modules[4096].source.len()
        );
    });
}

#[test]
fn native_module_graph_reports_original_sources_for_missing_cycles_and_invalid_modules() {
    support::worker(|| {
        let entry = "program entry use a fn main()->Field{7}";
        for (source, index, code, start, end) in [
            (b"module a use missing".as_slice(), 0, 3, 9, 20),
            (b"module a use entry", 0, 4, 9, 18),
            (b"module a use a", 0, 4, 9, 14),
            (b"module wrong", 0, 3, 7, 12),
            (b"program a", 0, 3, 0, 7),
            (b"module a use missing.", 0, 2, 21, 21),
            (b"module a\xff", 0, 1, 8, 9),
        ] {
            let modules = [module("a", source), module("entry", entry)];
            let graph = run(&modules, 4096, 0).unwrap();
            assert_eq!(
                (graph.index, graph.code, graph.start, graph.end),
                (index, code, start, end),
                "{graph:?}"
            );
            assert!(graph.order.is_empty());
        }
        let modules = [module("entry", "program entry use absent")];
        let graph = run(&modules, 4096, 0).unwrap();
        assert_eq!(
            (graph.index, graph.code, graph.start, graph.end),
            (0, 3, 14, 24)
        );
    });
}

#[test]
fn native_module_graph_keeps_one_allowance_and_checks_shared_source_and_use_caps() {
    support::worker(|| {
        let modules = [
            module("a", "module a"),
            module("entry", "program entry use a use a"),
        ];
        let graph = run(&modules, 4096, 0).unwrap();
        assert_eq!(graph.code, 0);
        let used = graph.admitted - graph.remaining;
        let exact = run(&modules, 4096, used).unwrap();
        assert_eq!((exact.code, exact.remaining), (0, 0));
        assert!(run(&modules, 4096, used - 1).is_err());
        assert_eq!(run(&modules, 2, 0).unwrap().code, 0);
        let modules = [
            module("a", "module a"),
            module("entry", "program entry use a use a use a"),
        ];
        let limited = run(&modules, 2, 0).unwrap();
        assert_eq!(
            (limited.index, limited.code, limited.start, limited.end),
            (1, 7, 26, 31)
        );
        let entry = "program entry use a";
        let exact = format!("module a //{}", " ".repeat(4096 - entry.len() - 11));
        assert_eq!(exact.len() + entry.len(), 4096);
        let modules = [module("a", &exact), module("entry", entry)];
        let graph = run(&modules, 4096, 0).unwrap();
        assert_eq!((graph.code, graph.bytes), (0, 4096));
        let modules = [module("a", format!("{exact} ")), module("entry", entry)];
        let graph = run(&modules, 4096, 0).unwrap();
        assert_eq!(
            (graph.index, graph.code, graph.start, graph.end),
            (0, 7, 0, 0)
        );
        assert_eq!(graph.bytes, entry.len());
    });
}
