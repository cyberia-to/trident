//! Paired constant export lookup over one immutable closure per test process.
#[allow(dead_code)]
#[path = "../examples/selfhost_data/model.rs"]
mod data;
#[path = "native_control/support.rs"]
mod support;
use nox::{artifact, sequential, Order, Outcome, Reduction};
use std::{collections::BTreeMap, path::Path, sync::OnceLock};
use trident::NATIVE_ARTIFACT_LIMITS as LIMITS;

const MODULE: &str = "std.compiler.nox.constant_exports";
const BEFORE: &str = include_str!("../audit/self-hosting/constant-export-row/before.tri");
type Arena = Reduction<{ 1 << 18 }>;

fn sources() -> &'static BTreeMap<String, String> {
    static SOURCES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    SOURCES.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        if std::env::var_os("TRIDENT_CONSTANT_EXPORT_CAPTURED_SOURCES").is_some() {
            let saved: serde_json::Value = serde_json::from_slice(
                &std::fs::read(root.join("audit/self-hosting/constant-export-row/sources.json"))
                    .unwrap(),
            )
            .unwrap();
            serde_json::from_value(saved["sources"].clone()).unwrap()
        } else {
            let inventory: serde_json::Value = serde_json::from_slice(
                &std::fs::read(root.join("audit/self-hosting/source-capacity-closure.json"))
                    .unwrap(),
            )
            .unwrap();
            inventory["modules"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(name, item)| {
                    (
                        name.clone(),
                        std::fs::read_to_string(root.join(item["path"].as_str().unwrap())).unwrap(),
                    )
                })
                .collect()
        }
    })
}

fn compile(source: &str, before: bool, compiler: bool) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.tri");
    std::fs::write(&path, source).unwrap();
    let mut module_sources = sources().clone();
    module_sources.remove("native_compiler");
    if before {
        module_sources.insert(MODULE.into(), BEFORE.into());
    }
    trident::compile_native_artifact_project(
        &path,
        &trident::CompileOptions {
            module_sources,
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

fn fixtures() -> &'static [Vec<u8>; 2] {
    static FIXTURES: OnceLock<[Vec<u8>; 2]> = OnceLock::new();
    FIXTURES.get_or_init(|| {
        let source = include_str!("fixtures/native_constant_export_rows.tri");
        [compile(source, true, false), compile(source, false, false)]
    })
}

fn words(arena: &mut Arena, values: &[u64], terminated: bool) -> Order {
    let (initial, values) = if terminated {
        (0, values)
    } else {
        (*values.last().unwrap(), &values[..values.len() - 1])
    };
    let mut result = data::atom(arena, initial).unwrap();
    for &value in values.iter().rev() {
        let value = data::atom(arena, value).unwrap();
        result = data::pair(arena, value, result).unwrap();
    }
    result
}

struct Case {
    source: String,
    caller: String,
    rows: Vec<[u64; 8]>,
    owner: u64,
    query: [u64; 2],
    expected: Option<[u64; 9]>,
}

#[derive(Debug, serde::Serialize)]
struct Observation {
    reductions: u64,
    loaded_nodes: u32,
    nodes: u32,
    frames: u32,
    output_blake3: String,
}

fn run(program: &[u8], case: &Case) -> Observation {
    let mut arena = Arena::try_new_boxed().unwrap();
    assert!(arena.limit_allocations(196608));
    let root = artifact::decode(&mut arena, program, LIMITS).unwrap();
    let loaded_nodes = arena.count();
    let mut fields = arena.tail(root).unwrap();
    for _ in 0..3 {
        fields = arena.tail(fields).unwrap();
    }
    let formula = arena.head(fields).unwrap();
    let source = data::Bytes::from_slice(&mut arena, case.source.as_bytes(), 65536)
        .unwrap()
        .encode(&mut arena)
        .unwrap();
    let caller = data::Bytes::from_slice(&mut arena, case.caller.as_bytes(), 65536)
        .unwrap()
        .encode(&mut arena)
        .unwrap();
    let rows: Vec<_> = case
        .rows
        .iter()
        .map(|r| words(&mut arena, r, true))
        .collect();
    let rows = data::Seq::from_values(&mut arena, &rows, 4096)
        .unwrap()
        .encode(&mut arena)
        .unwrap();
    let query = words(
        &mut arena,
        &[case.owner, case.query[0], case.query[1]],
        false,
    );
    let rest = data::pair(&mut arena, rows, query).unwrap();
    let rest = data::pair(&mut arena, caller, rest).unwrap();
    let input = data::pair(&mut arena, source, rest).unwrap();
    let execution = sequential::reduce_cached(
        &mut arena,
        input,
        formula,
        20_000_000,
        sequential::Limits { max_frames: 65536 },
    )
    .unwrap();
    let Outcome::Ok(result, remaining) = execution.outcome else {
        panic!("{:?}", execution.outcome)
    };
    let nodes = arena.count();
    let output = artifact::encode(&arena, result, LIMITS).unwrap();
    let expected = match case.expected {
        Some(values) => words(&mut arena, &values, false),
        None => data::atom(&mut arena, 0).unwrap(),
    };
    assert_eq!(output, artifact::encode(&arena, expected, LIMITS).unwrap());
    Observation {
        reductions: 20_000_000 - remaining,
        loaded_nodes,
        nodes,
        frames: execution.peak_frames,
        output_blake3: blake3::hash(&output).to_hex().to_string(),
    }
}

#[test]
fn paired_lookup_keeps_final_names_foreign_literal_provenance_and_misses() {
    support::worker(|| {
        let mut cases = Vec::new();
        for (name, owner, expected) in [
            ("A", 0, Some([4, 0, 4, 5, 3, 4294967295, 2, 9001, 9011])),
            ("B", 0, Some([2, 0, 6, 7, 0, 77, 1, 5120, 5122])),
            ("A", 1, Some([3, 1, 4, 5, 0, 31, 0, 100, 102])),
            ("Z", 0, None),
            ("A", 2, None),
        ] {
            cases.push(Case {
                source: "pad A B C".into(),
                caller: format!("unequal offset {name}"),
                rows: vec![
                    [0, 4, 5, 0, 11, 0, 10, 12],
                    [0, 6, 7, 0, 77, 1, 5120, 5122],
                    [1, 4, 5, 0, 31, 0, 100, 102],
                    [0, 4, 5, 3, 4294967295, 2, 9001, 9011],
                    [0, 8, 9, 1, 0, 0, 42, 43],
                ],
                owner,
                query: [15, 16],
                expected,
            });
        }
        for length in [7, 8, 255, 256, 300] {
            let name = "x".repeat(length);
            for matched in [true, false] {
                let caller = if matched {
                    name.clone()
                } else {
                    format!("{}y", "x".repeat(length - 1))
                };
                cases.push(Case {
                    source: format!("{}{}", " ".repeat(5000), name),
                    caller,
                    rows: vec![[0, 5000, 5000 + length as u64, 0, 9, 2, 65000, 65020]],
                    owner: 0,
                    query: [0, length as u64],
                    expected: matched.then_some([
                        1,
                        0,
                        5000,
                        5000 + length as u64,
                        0,
                        9,
                        2,
                        65000,
                        65020,
                    ]),
                });
            }
        }
        let source = (0..32).map(|i| format!("N{i:03} ")).collect::<String>();
        let rows: Vec<_> = (0..32)
            .map(|i| [0, 5 * i, 5 * i + 4, 0, i, 2, 1000 + i, 1001 + i])
            .collect();
        for (caller, expected) in [
            ("N000", Some([1, 0, 0, 4, 0, 0, 2, 1000, 1001])),
            ("M000", None),
        ] {
            cases.push(Case {
                source: source.clone(),
                caller: caller.into(),
                rows: rows.clone(),
                owner: 0,
                query: [0, 4],
                expected,
            });
        }
        let observations: Vec<_> = cases
            .iter()
            .enumerate()
            .map(|(index, case)| {
                let before = run(&fixtures()[0], case);
                let after = run(&fixtures()[1], case);
                assert_eq!(before.output_blake3, after.output_blake3);
                // An empty owner's head never visits a row and has no row work
                // to remove. Resource savings apply only to nonempty searches.
                if case.rows.iter().any(|row| row[0] == case.owner) {
                    assert!(after.reductions < before.reductions, "case {index}");
                }
                serde_json::json!({"case":index,"query":case.query,"owner":case.owner,
                "source_bytes":case.source.len(),"rows":case.rows.len(),
                "before":before,"after":after})
            })
            .collect();
        println!("{}", serde_json::json!({"component":observations}));
    });
}

#[test]
#[ignore = "explicit paired complete C1 footprint diagnostic"]
fn constant_export_row_compiler_footprint() {
    support::worker(|| {
        let results: Vec<_> = [true, false].into_iter().map(|before| {
            let artifact = compile(&sources()["native_compiler"], before, true);
            let mut arena = Reduction::<{ 1 << 20 }>::try_new_boxed().unwrap();
            artifact::decode(&mut arena, &artifact, LIMITS).unwrap();
            serde_json::json!({"before":before,"artifact_bytes":artifact.len(),
                "loaded_nodes":arena.count(),"artifact_blake3":blake3::hash(&artifact).to_hex().to_string()})
        }).collect();
        println!("{}", serde_json::json!({"compiler":results}));
    });
}
