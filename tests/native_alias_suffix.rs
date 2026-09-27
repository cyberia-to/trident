//! Paired native alias matching over one immutable source snapshot.
#[allow(dead_code)]
#[path = "../examples/selfhost_data/model.rs"]
mod model;
#[path = "native_control/support.rs"]
mod support;
use nox::{artifact, sequential, Outcome, Reduction};
use std::{collections::BTreeMap, path::Path, sync::OnceLock};
use trident::NATIVE_ARTIFACT_LIMITS as LIMITS;

struct Snapshot {
    before: BTreeMap<String, String>,
    after: BTreeMap<String, String>,
    entry: String,
}

fn snapshot() -> &'static Snapshot {
    static SOURCES: OnceLock<Snapshot> = OnceLock::new();
    SOURCES.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        // Ordinary regressions follow production. Audit runs select the saved
        // immutable source map so concurrent unrelated edits cannot change it.
        let captured: BTreeMap<String, String> =
            if std::env::var_os("TRIDENT_ALIAS_CAPTURED_SOURCES").is_some() {
                let saved: serde_json::Value = serde_json::from_slice(
                    &std::fs::read(
                        root.join("audit/self-hosting/alias-suffix-source-snapshot.json"),
                    )
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
                            std::fs::read_to_string(root.join(item["path"].as_str().unwrap()))
                                .unwrap(),
                        )
                    })
                    .collect()
            };
        let mut after = BTreeMap::new();
        let mut entry = String::new();
        for (name, source) in captured {
            if name == "native_compiler" {
                entry = source;
            } else {
                after.insert(name, source);
            }
        }
        let saved: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("audit/self-hosting/alias-suffix-before.json")).unwrap(),
        )
        .unwrap();
        let mut before = after.clone();
        for (path, source) in saved["sources"].as_object().unwrap() {
            let name = path
                .trim_start_matches("lib/")
                .trim_end_matches(".tri")
                .replace('/', ".");
            before.insert(name, source.as_str().unwrap().into());
        }
        Snapshot {
            before,
            after,
            entry,
        }
    })
}

fn compile(source: &str, sources: BTreeMap<String, String>, compiler: bool) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.tri");
    std::fs::write(&path, source).unwrap();
    let options = trident::CompileOptions {
        module_sources: sources,
        ..Default::default()
    };
    trident::compile_native_artifact_project(
        &path,
        &options,
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
        let frozen = snapshot();
        let mut before = frozen.before.clone();
        // Transplant the exact former private matcher into the same public
        // probe entry. The production C1 comparison below uses unmodified old
        // modules; this transplant is only the isolated component benchmark.
        let imports = &before["std.compiler.nox.function_imports"];
        let start = imports.find("#[pure]\nfn alias(").unwrap();
        let end = start + imports[start + 1..].find("\n#[pure]\n").unwrap() + 1;
        let old = imports[start..end].replace("fn alias(", "pub fn matches_owner(");
        let qualified = before.get_mut("std.compiler.nox.qualified_name").unwrap();
        *qualified = qualified.replacen(
            "\n\n",
            "\n\nuse vm.nox.noun\nuse std.compiler.nox.ascii\n",
            1,
        );
        qualified.push_str(&old);
        let fixture = include_str!("fixtures/native_alias_suffix.tri");
        [
            compile(fixture, before, false),
            compile(fixture, frozen.after.clone(), false),
        ]
    })
}

#[derive(Debug, serde::Serialize)]
struct Observation {
    matched: bool,
    reductions: u64,
    loaded_nodes: u32,
    nodes: u32,
    frames: u32,
}

fn run(compiled: &[u8], prefix: &[u8], owner: &[u8]) -> Observation {
    let mut arena = Reduction::<{ 1 << 18 }>::try_new_boxed().unwrap();
    assert!(arena.limit_allocations(196608));
    let program = artifact::decode(&mut arena, compiled, LIMITS).unwrap();
    let loaded_nodes = arena.count();
    let mut fields = arena.tail(program).unwrap();
    for _ in 0..3 {
        fields = arena.tail(fields).unwrap();
    }
    let formula = arena.head(fields).unwrap();
    let prefix = model::Bytes::from_slice(&mut arena, prefix, 255)
        .unwrap()
        .encode(&mut arena)
        .unwrap();
    let owner = model::Bytes::from_slice(&mut arena, owner, 255)
        .unwrap()
        .encode(&mut arena)
        .unwrap();
    let subject = model::pair(&mut arena, prefix, owner).unwrap();
    let execution = sequential::reduce_cached(
        &mut arena,
        subject,
        formula,
        20_000_000,
        sequential::Limits { max_frames: 65536 },
    )
    .unwrap();
    let Outcome::Ok(result, remaining) = execution.outcome else {
        panic!("{:?}", execution.outcome)
    };
    Observation {
        matched: arena.atom_value(result).unwrap().as_u64() == 1,
        reductions: 20_000_000 - remaining,
        loaded_nodes,
        nodes: arena.count(),
        frames: execution.peak_frames,
    }
}

fn verify(prefix: &[u8], owner: &[u8]) {
    let expected = prefix == owner || prefix == owner.rsplit(|b| *b == b'.').next().unwrap();
    for fixture in fixtures() {
        assert_eq!(
            run(fixture, prefix, owner).matched,
            expected,
            "{prefix:?} / {owner:?}"
        );
    }
}

#[test]
fn suffix_matching_preserves_full_names_basename_boundaries_and_all_bytes() {
    support::worker(|| {
        for (prefix, owner) in [
            ("", ""),
            ("", "a"),
            ("", "a."),
            ("a", "a"),
            ("a", "b.a"),
            ("a", "ba"),
            ("a", "a.b"),
            ("field", "vm.core.field"),
            ("core.field", "vm.core.field"),
            ("vm.core.field", "vm.core.field"),
            (".field", "vm.core.field"),
            ("a.b", "x.a.b"),
            ("a..b", "x.a..b"),
        ] {
            verify(prefix.as_bytes(), owner.as_bytes());
        }
        for length in [
            1, 2, 3, 4, 5, 7, 8, 15, 16, 17, 31, 32, 63, 64, 127, 128, 253, 254, 255,
        ] {
            let owner = vec![b'x'; length];
            verify(&owner, &owner);
            if length > 1 {
                let mut dotted = owner.clone();
                dotted[0] = b'.';
                verify(&dotted[1..], &dotted);
                verify(&owner[1..], &owner);
                let mut changed = dotted[1..].to_vec();
                changed[0] = b'y';
                verify(&changed, &dotted);
            }
        }
        for lane in 0..4 {
            for byte in 0..=255u8 {
                let mut prefix = *b"xxxx";
                prefix[lane] = byte;
                let mut owner = b"module.xxxx".to_vec();
                owner[7 + lane] = byte;
                verify(&prefix, &owner);
            }
        }
    });
}

#[test]
fn alias_suffix_resource_probe() {
    support::worker(|| {
        let mut observations = Vec::new();
        for (prefix, owner) in [
            ("noun".to_owned(), "vm.nox.noun".to_owned()),
            (
                "noun".to_owned(),
                "std.compiler.nox.qualified_name".to_owned(),
            ),
            ("field".to_owned(), "vm.core.field".to_owned()),
            ("core.field".to_owned(), "vm.core.field".to_owned()),
            ("std.nox.bytes".to_owned(), "std.nox.bytes".to_owned()),
            ("x".to_owned(), format!("{}.x", "p".repeat(253))),
            ("x".repeat(253), format!("p.{}", "x".repeat(253))),
        ] {
            observations.push(serde_json::json!({"prefix":prefix,"owner":owner,
                "before":run(&fixtures()[0],prefix.as_bytes(),owner.as_bytes()),
                "after":run(&fixtures()[1],prefix.as_bytes(),owner.as_bytes())}));
        }
        println!("{}", serde_json::json!({"component":observations}));
    });
}

#[test]
#[ignore = "explicit paired complete C1 footprint diagnostic"]
fn alias_suffix_compiler_footprint() {
    support::worker(|| {
        let frozen = snapshot();
        let mut results = Vec::new();
        for (variant, sources) in [("before", &frozen.before), ("after", &frozen.after)] {
            let compiled = compile(&frozen.entry, sources.clone(), true);
            let mut arena = Reduction::<{ 1 << 20 }>::try_new_boxed().unwrap();
            artifact::decode(&mut arena, &compiled, LIMITS).unwrap();
            let hashes: BTreeMap<_, _> = sources
                .iter()
                .map(|(name, source)| (name, blake3::hash(source.as_bytes()).to_hex().to_string()))
                .collect();
            results.push(serde_json::json!({"variant":variant,"source_blake3":hashes,
                "entry_blake3":blake3::hash(frozen.entry.as_bytes()).to_hex().to_string(),
                "artifact_bytes":compiled.len(),"loaded_nodes":arena.count(),
                "artifact_blake3":blake3::hash(&compiled).to_hex().to_string()}));
        }
        println!("{}", serde_json::json!({"compiler":results}));
    });
}
