// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::*;

#[test]
fn test_json_string_escaping() {
    assert_eq!(json_string("hello"), "\"hello\"");
    assert_eq!(json_string("a\"b"), "\"a\\\"b\"");
    assert_eq!(json_string("a\\b"), "\"a\\\\b\"");
    assert_eq!(json_string("a\nb"), "\"a\\nb\"");
}

#[test]
fn test_iso8601_now_format() {
    let ts = iso8601_now();
    // Should match pattern: YYYY-MM-DDTHH:MM:SSZ
    assert!(ts.ends_with('Z'));
    assert_eq!(ts.len(), 20);
    assert_eq!(&ts[4..5], "-");
    assert_eq!(&ts[7..8], "-");
    assert_eq!(&ts[10..11], "T");
    assert_eq!(&ts[13..14], ":");
    assert_eq!(&ts[16..17], ":");
}

#[test]
fn test_days_to_date_epoch() {
    let (y, m, d) = days_to_date(0);
    assert_eq!((y, m, d), (1970, 1, 1));
}

#[test]
fn test_days_to_date_known() {
    // 2026-02-11 is day 20,495 since epoch
    let (y, m, d) = days_to_date(20495);
    assert_eq!((y, m, d), (2026, 2, 11));
}

#[test]
fn test_manifest_to_json_structure() {
    let manifest = PackageManifest {
        name: "test".to_string(),
        version: "0.1.0".to_string(),
        program_digest: "aabb".to_string(),
        source_hash: "ccdd".to_string(),
        target_vm: "triton".to_string(),
        target_os: Some("neptune".to_string()),
        architecture: "stack".to_string(),
        cost: ManifestCost {
            table_values: vec![100, 50, 25, 0, 0, 0],
            table_names: vec![
                "processor".into(),
                "hash".into(),
                "u32".into(),
                "op_stack".into(),
                "ram".into(),
                "jump_stack".into(),
            ],
            padded_height: 256,
        },
        functions: vec![ManifestFunction {
            name: "main".to_string(),
            hash: "eeff".to_string(),
            signature: "fn main()".to_string(),
        }],
        entry_point: "main".to_string(),
        built_at: "2026-02-11T00:00:00Z".to_string(),
        compiler_version: "0.1.0".to_string(),
    };

    let json = manifest.to_json();
    assert!(json.contains("\"name\": \"test\""));
    assert!(json.contains("\"program_digest\": \"aabb\""));
    assert!(json.contains("\"os\": \"neptune\""));
    assert!(json.contains("\"vm\": \"triton\""));
    assert!(json.contains("\"processor\": 100"));
    assert!(json.contains("\"padded_height\": 256"));
    assert!(json.contains("\"entry_point\": \"main\""));
    assert!(json.contains("\"fn main()\""));
}

#[test]
fn test_manifest_null_os() {
    let manifest = PackageManifest {
        name: "bare".to_string(),
        version: "0.1.0".to_string(),
        program_digest: "aa".to_string(),
        source_hash: "bb".to_string(),
        target_vm: "triton".to_string(),
        target_os: None,
        architecture: "stack".to_string(),
        cost: ManifestCost {
            table_values: vec![0, 0, 0, 0, 0, 0],
            table_names: vec![
                "processor".into(),
                "hash".into(),
                "u32".into(),
                "op_stack".into(),
                "ram".into(),
                "jump_stack".into(),
            ],
            padded_height: 0,
        },
        functions: vec![],
        entry_point: "main".to_string(),
        built_at: "2026-01-01T00:00:00Z".to_string(),
        compiler_version: "0.1.0".to_string(),
    };

    let json = manifest.to_json();
    assert!(json.contains("\"os\": null"));
}

#[test]
fn test_program_digest_deterministic() {
    let tasm = "push 1\npush 2\nadd\nwrite_io 1\nhalt\n";
    let hash1 = ContentHash(crate::poseidon2::hash_bytes(tasm.as_bytes()));
    let hash2 = ContentHash(crate::poseidon2::hash_bytes(tasm.as_bytes()));
    assert_eq!(hash1.to_hex(), hash2.to_hex());
}

#[test]
fn test_generate_artifact_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let source = "program test\nfn main() {\n    pub_write(pub_read())\n}\n";
    let filename = "test.tri";

    // Parse the source
    let file = crate::parse_source_silent(source, filename).unwrap();

    // Create a minimal cost
    let cost = crate::cost::CostAnalyzer::default().analyze_file(&file);

    let target_vm = TerrainConfig::triton();
    let tasm = "push 1\nwrite_io 1\nhalt\n";

    let result = generate_artifact(
        "test",
        "0.1.0",
        tasm,
        &file,
        &cost,
        &target_vm,
        None,
        dir.path(),
    )
    .unwrap();

    // Verify directory and files exist
    assert!(result.artifact_dir.exists());
    assert!(result.tasm_path.exists());
    assert!(result.manifest_path.exists());
    assert_eq!(
        result.artifact_dir.file_name().unwrap().to_str().unwrap(),
        "test.deploy"
    );

    // Verify TASM content
    let written_tasm = std::fs::read_to_string(&result.tasm_path).unwrap();
    assert_eq!(written_tasm, tasm);

    // Verify manifest content
    let manifest_json = std::fs::read_to_string(&result.manifest_path).unwrap();
    assert!(manifest_json.contains("\"name\": \"test\""));
    assert!(manifest_json.contains("\"program_digest\""));
    assert!(manifest_json.contains("\"source_hash\""));
    assert!(manifest_json.contains("\"vm\": \"triton\""));

    // Verify digest is non-empty
    assert!(!result.manifest.program_digest.is_empty());
    assert!(!result.manifest.source_hash.is_empty());
}
