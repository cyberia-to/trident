//! Paired C1 footprint measurement over one captured compiler source closure.
#[allow(dead_code)]
#[path = "../examples/selfhost_data/model.rs"]
mod data;
#[allow(dead_code)]
#[path = "../examples/selfhost_jobs/schema.rs"]
mod schema;
#[path = "native_control/support.rs"]
mod support;
#[allow(dead_code)]
#[path = "../examples/selfhost_jobs/validate.rs"]
mod validate;
use nox::{artifact, sequential, Outcome, Reduction};
use std::{collections::BTreeMap, path::Path};
use trident::NATIVE_ARTIFACT_LIMITS as LIMITS;

fn body_chunks(compiler: &[u8]) -> serde_json::Value {
    // These are the unchanged native_compiler default caps and exact body case.
    let caps = [
        4096,
        128,
        16,
        4096,
        1_000_000,
        16_777_216,
        196608,
        4096,
        100_000_000,
        196608,
        65536,
    ];
    let mut arena = Reduction::<{ 1 << 18 }>::try_new_boxed().unwrap();
    assert!(arena.limit_allocations(caps[9] as u32));
    let c1 = artifact::decode(&mut arena, compiler, LIMITS).unwrap();
    let modules = [schema::Module {
        path: "sample".into(),
        origin: "pilot".into(),
        version: "1".into(),
        source: format!(
            "program sample fn main() -> Field {{ let mut x=0 {}x }}",
            "x=x+1 ".repeat(9)
        )
        .into_bytes(),
    }];
    let options = schema::Options {
        input: 0,
        output: 0,
        optimization: 0,
        cfg: vec![],
    };
    let job = schema::job(&mut arena, c1, &modules, "sample", "main", &options, &caps).unwrap();
    let mut host = caps;
    host[0] = 4_194_304;
    host[3] = 65_536;
    host[4] = 16 << 20;
    let admitted = validate::job(&mut arena, job, c1, host).unwrap();
    let mut fields = arena.tail(c1).unwrap();
    for _ in 0..3 {
        fields = arena.tail(fields).unwrap();
    }
    let formula = arena.head(fields).unwrap();
    let execution = sequential::reduce_cached(
        &mut arena,
        job,
        formula,
        caps[8],
        sequential::Limits {
            max_frames: caps[10] as u32,
        },
    )
    .unwrap();
    let nodes = arena.count();
    match execution.outcome {
        Outcome::Ok(result, remaining) => {
            let validate::ResultValue::Success {
                artifact: program, ..
            } = validate::result(&mut arena, result, job, &admitted).unwrap()
            else {
                panic!("unexpected guest diagnostic")
            };
            let bytes = artifact::encode(&arena, program, LIMITS).unwrap();
            let value = support::run(&bytes, 0, 1_000_000, 65536, 196608).unwrap().0;
            assert_eq!(value, 9);
            serde_json::json!({"status": "returned", "value": value, "nodes": nodes,
                "reductions": caps[8] - remaining, "frames": execution.peak_frames})
        }
        other => serde_json::json!({"status": "failed", "outcome": format!("{other:?}"),
            "nodes": nodes, "frames": execution.peak_frames}),
    }
}

#[test]
#[ignore = "explicit paired compiler footprint diagnostic"]
fn discarded_checked_casts_preserve_a_smaller_compiler_artifact() {
    support::worker(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let inventory: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("audit/self-hosting/source-capacity-closure.json")).unwrap(),
        )
        .unwrap();
        let mut options = trident::CompileOptions::default();
        let mut sources = BTreeMap::new();
        let mut main = String::new();
        for (name, entry) in inventory["modules"].as_object().unwrap() {
            let source =
                std::fs::read_to_string(root.join(entry["path"].as_str().unwrap())).unwrap();
            sources.insert(
                name.clone(),
                blake3::hash(source.as_bytes()).to_hex().to_string(),
            );
            if name == "native_compiler" {
                main = source;
            } else {
                options.module_sources.insert(name.clone(), source);
            }
        }
        let after = options.module_sources["std.nox.bytes"].clone();
        let saved: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("audit/self-hosting/byte-discard-before.json")).unwrap(),
        )
        .unwrap();
        let before = saved["before_source"].as_str().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let entry = dir.path().join("main.tri");
        std::fs::write(&entry, main).unwrap();
        let mut reports = Vec::new();
        for (variant, bytes_source) in [("before", before), ("after", after.as_str())] {
            options
                .module_sources
                .insert("std.nox.bytes".into(), bytes_source.into());
            let compiled = trident::compile_native_artifact_project(
                &entry,
                &options,
                trident::NativeArtifactProfile::CompilerJob,
                LIMITS,
            )
            .unwrap();
            let mut arena = Reduction::<{ 1 << 20 }>::try_new_boxed().unwrap();
            artifact::decode(&mut arena, &compiled.bytes, LIMITS).unwrap();
            reports.push(serde_json::json!({
                "variant": variant, "bytes_source_blake3": blake3::hash(bytes_source.as_bytes()).to_hex().to_string(),
                "artifact_bytes": compiled.bytes.len(), "loaded_nodes": arena.count(),
                "artifact_blake3": blake3::hash(&compiled.bytes).to_hex().to_string(),
                "particle": compiled.particle.iter().map(|b| format!("{b:02x}")).collect::<String>(),
                "body_chunks_default_caps": body_chunks(&compiled.bytes),
            }));
        }
        println!(
            "{}",
            serde_json::json!({"captured_sources_blake3": sources, "variants": reports})
        );
        assert!(
            reports[1]["artifact_bytes"].as_u64().unwrap()
                < reports[0]["artifact_bytes"].as_u64().unwrap()
        );
        assert_eq!(reports[1]["body_chunks_default_caps"]["status"], "returned");
        assert!(
            reports[1]["loaded_nodes"].as_u64().unwrap()
                < reports[0]["loaded_nodes"].as_u64().unwrap()
        );
    });
}
