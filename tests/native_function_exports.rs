//! Compare owner-scoped export lookup with the retained implementation.
#[allow(dead_code)]
#[path = "../examples/selfhost_data/model.rs"]
mod model;
#[path = "native_control/support.rs"]
mod support;
use nox::{artifact, sequential, Order, Outcome, Reduction};
use std::{collections::BTreeMap, path::Path, sync::OnceLock};
use trident::NATIVE_ARTIFACT_LIMITS as LIMITS;

const OWNER: &str = "std.compiler.nox.function_exports";
const MODULE: &str = "lib/std/compiler/nox/function_exports.tri";
const BUDGET: u64 = 100_000_000;
type Arena = Reduction<{ 1 << 20 }>;

fn captured_sources() -> BTreeMap<String, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let snapshot: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("audit/self-hosting/alias-suffix-source-snapshot.json")).unwrap(),
    )
    .unwrap();
    serde_json::from_value(snapshot["sources"].clone()).unwrap()
}

fn compile(source: &str, sources: BTreeMap<String, String>, compiler: bool) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("main.tri");
    std::fs::write(&entry, source).unwrap();
    trident::compile_native_artifact_project(
        &entry,
        &trident::CompileOptions {
            module_sources: sources,
            ..Default::default()
        },
        if compiler {
            trident::NativeArtifactProfile::CompilerJob
        } else {
            trident::NativeArtifactProfile::RawNoun
        },
        LIMITS,
    )
    .unwrap()
    .bytes
}

fn variants() -> [BTreeMap<String, String>; 2] {
    let mut captured = captured_sources();
    captured.remove("native_compiler");
    let before = captured[OWNER].clone();
    let current =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(MODULE)).unwrap();
    let base = if std::env::var_os("TRIDENT_EXPORT_FROZEN_SNAPSHOT").is_some() {
        captured
    } else {
        BTreeMap::new()
    };
    let mut old = base.clone();
    old.insert(OWNER.into(), before);
    let mut new = base;
    new.insert(OWNER.into(), current);
    [old, new]
}

fn artifacts() -> &'static [Vec<u8>; 2] {
    static COMPILED: OnceLock<[Vec<u8>; 2]> = OnceLock::new();
    COMPILED.get_or_init(|| {
        variants().map(|sources| {
            compile(
                include_str!("fixtures/native_function_exports.tri"),
                sources,
                false,
            )
        })
    })
}

fn pair(ar: &mut Arena, a: Order, b: Order) -> Order {
    model::pair(ar, a, b).unwrap()
}
fn atom(ar: &mut Arena, value: u32) -> Order {
    model::atom(ar, value.into()).unwrap()
}
fn span(ar: &mut Arena, start: u32, end: u32) -> Order {
    let start = atom(ar, start);
    let end = atom(ar, end);
    pair(ar, start, end)
}

#[derive(Debug, serde::Serialize)]
struct Measurement {
    result: (u64, u64),
    reductions: u64,
    loaded_nodes: u32,
    nodes: u32,
    frames: u32,
}

// Rows are (owner, name start, name end, target), published in source order.
fn run(
    artifact: &[u8],
    source: &[u8],
    caller: &[u8],
    owner: u32,
    query: (u32, u32),
    rows: &[(u32, u32, u32, u32)],
) -> Measurement {
    let mut ar = Arena::try_new_boxed().unwrap();
    assert!(ar.limit_allocations(786432));
    let program = artifact::decode(&mut ar, artifact, LIMITS).unwrap();
    let loaded_nodes = ar.count();
    let mut fields = ar.tail(program).unwrap();
    for _ in 0..3 {
        fields = ar.tail(fields).unwrap();
    }
    let formula = ar.head(fields).unwrap();
    let mut list = atom(&mut ar, 0);
    for &(owner, start, end, target) in rows.iter().rev() {
        let owner = atom(&mut ar, owner);
        let name = span(&mut ar, start, end);
        let target = atom(&mut ar, target);
        let rest = pair(&mut ar, name, target);
        let row = pair(&mut ar, owner, rest);
        list = pair(&mut ar, row, list);
    }
    let query = span(&mut ar, query.0, query.1);
    let rest = pair(&mut ar, query, list);
    let owner = atom(&mut ar, owner);
    let request = pair(&mut ar, owner, rest);
    let caller = model::Bytes::from_slice(&mut ar, caller, 65536)
        .unwrap()
        .encode(&mut ar)
        .unwrap();
    let args = pair(&mut ar, caller, request);
    let source = model::Bytes::from_slice(&mut ar, source, 65536)
        .unwrap()
        .encode(&mut ar)
        .unwrap();
    let input = pair(&mut ar, source, args);
    let execution = sequential::reduce_cached(
        &mut ar,
        input,
        formula,
        BUDGET,
        sequential::Limits { max_frames: 65536 },
    )
    .unwrap();
    let Outcome::Ok(result, remaining) = execution.outcome else {
        panic!("{:?}", execution.outcome)
    };
    Measurement {
        result: (
            model::value(&ar, ar.head(result).unwrap()).unwrap(),
            model::value(&ar, ar.tail(result).unwrap()).unwrap(),
        ),
        reductions: BUDGET - remaining,
        loaded_nodes,
        nodes: ar.count(),
        frames: execution.peak_frames,
    }
}

#[test]
fn export_lookup_preserves_owner_final_binding_and_separate_source_spans() {
    support::worker(|| {
        let source = b"alpha beta alpha suffix";
        let rows = [
            (1, 0, 5, 12),
            (2, 0, 5, 15),
            (1, 6, 10, 0),
            (1, 11, 16, 4095),
        ];
        for (owner, query, expected) in [
            (1, "alpha", (1, 4095)),
            (2, "alpha", (1, 15)),
            (1, "beta", (1, 0)),
            (3, "alpha", (0, 0)),
            (1, "alph", (0, 0)),
            (1, "alphx", (0, 0)),
            (1, "suffix", (0, 0)),
        ] {
            let caller = format!("xx{query}!");
            for artifact in artifacts() {
                let result = run(
                    artifact,
                    source,
                    caller.as_bytes(),
                    owner,
                    (2, 2 + query.len() as u32),
                    &rows,
                );
                assert_eq!(result.result, expected, "{owner}/{query}");
            }
        }
        let source = vec![b'a'; 511];
        let mut caller = vec![b'a'; 512];
        caller[0] = b'x';
        for artifact in artifacts() {
            assert_eq!(
                run(artifact, &source, &caller, 7, (1, 512), &[(7, 0, 511, 42)]).result,
                (1, 42)
            );
            assert_eq!(run(artifact, b"", b"", 0, (0, 0), &[]).result, (0, 0));
        }
    });
}

#[test]
fn export_lookup_resource_probe() {
    support::worker(|| {
        let source = (0..32).map(|i| format!("name{i:02} ")).collect::<String>();
        let rows: Vec<_> = (0..32).map(|i| (1, i * 7, i * 7 + 6, i)).collect();
        let mut observations = Vec::new();
        for query in ["name00", "name16", "name31", "absent"] {
            let measures: Vec<_> = artifacts()
                .iter()
                .map(|artifact| {
                    run(
                        artifact,
                        source.as_bytes(),
                        query.as_bytes(),
                        1,
                        (0, 6),
                        &rows,
                    )
                })
                .collect();
            assert_eq!(measures[0].result, measures[1].result);
            assert!(measures[1].reductions < measures[0].reductions);
            observations
                .push(serde_json::json!({"query":query,"before":measures[0],"after":measures[1]}));
        }
        println!("{}", serde_json::json!({"export_lookup":observations}));
    });
}

#[test]
#[ignore = "explicit paired complete C1 footprint diagnostic"]
fn export_lookup_compiler_footprint() {
    support::worker(|| {
        assert!(std::env::var_os("TRIDENT_EXPORT_FROZEN_SNAPSHOT").is_some());
        let entry = captured_sources().remove("native_compiler").unwrap();
        let mut results = Vec::new();
        for (variant, sources) in ["before", "after"].into_iter().zip(variants()) {
            let source_hash = blake3::hash(sources[OWNER].as_bytes()).to_hex().to_string();
            let bytes = compile(&entry, sources, true);
            let mut ar = Arena::try_new_boxed().unwrap();
            artifact::decode(&mut ar, &bytes, LIMITS).unwrap();
            results.push(
                serde_json::json!({"variant":variant,"source_blake3":source_hash,
                "artifact_bytes":bytes.len(),"loaded_nodes":ar.count(),
                "artifact_blake3":blake3::hash(&bytes).to_hex().to_string()}),
            );
        }
        println!("{}", serde_json::json!({"compiler":results}));
    });
}
