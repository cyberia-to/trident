// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Program bundle: self-contained compilation artifact for warrior consumption.
//!
//! Contains the compiled assembly, metadata, cost analysis, and function
//! signatures. Warriors deserialize this from a JSON file or receive it
//! via the Rust API.

// ─── Data Types ────────────────────────────────────────────────────

/// Self-contained compilation artifact that a warrior needs to execute,
/// prove, or deploy a Trident program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramBundle {
    /// Program name (from project or filename).
    pub name: String,
    /// Program version.
    pub version: String,
    /// Target VM name (e.g. "triton", "miden").
    pub target_vm: String,
    /// Target OS name, if any (e.g. "neptune").
    pub target_os: Option<String>,
    /// Compiled assembly text (TASM for Triton, MASM for Miden, etc.).
    pub assembly: String,
    /// Entry point function name.
    pub entry_point: String,
    /// Function signatures with content hashes.
    pub functions: Vec<BundleFunction>,
    /// Cost analysis summary.
    pub cost: BundleCost,
    /// Content hash of the source AST (hex).
    pub source_hash: String,
    /// The program reads persistent state (nox: look pattern over BBG).
    /// The runner must supply the state and cons its root onto the subject
    /// (`[root_tree [params…]]`). Absent in JSON = false — old bundles and
    /// old readers are unaffected.
    pub reads_state: bool,
}

/// Function metadata within a bundle.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BundleFunction {
    pub name: String,
    pub hash: String,
    pub signature: String,
}

/// Cost analysis summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleCost {
    /// Cost values per table.
    pub table_values: Vec<u64>,
    /// Table names (e.g. ["processor", "hash", "u32", ...]).
    pub table_names: Vec<String>,
    /// Padded trace height (next power of two).
    pub padded_height: u64,
    /// Estimated proving time in nanoseconds.
    pub estimated_proving_ns: u64,
}

#[path = "artifact_json.rs"]
mod json;

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn sample_bundle() -> ProgramBundle {
        ProgramBundle {
            name: "test_program".to_string(),
            version: "0.1.0".to_string(),
            target_vm: "triton".to_string(),
            target_os: Some("neptune".to_string()),
            assembly: "    call main\n    halt\nmain:\n    push 42\n    return\n".to_string(),
            entry_point: "main".to_string(),
            functions: vec![BundleFunction {
                name: "main".to_string(),
                hash: "abc123".to_string(),
                signature: "fn main() -> Field".to_string(),
            }],
            cost: BundleCost {
                table_values: vec![100, 50, 10],
                table_names: vec![
                    "processor".to_string(),
                    "hash".to_string(),
                    "u32".to_string(),
                ],
                padded_height: 128,
                estimated_proving_ns: 1_000_000,
            },
            source_hash: "deadbeef".to_string(),
            reads_state: false,
        }
    }

    #[test]
    fn reads_state_roundtrips_when_true() {
        let mut bundle = sample_bundle();
        bundle.reads_state = true;
        let json = bundle.to_json();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&json).unwrap()["reads_state"],
            true
        );
        let parsed = ProgramBundle::from_json(&json).expect("parse failed");
        assert!(parsed.reads_state);
    }

    #[test]
    fn bundle_json_roundtrip() {
        let bundle = sample_bundle();
        let json = bundle.to_json();
        assert!(
            !json.contains("reads_state"),
            "reads_state must be absent when false (backward-compatible JSON)"
        );
        let parsed = ProgramBundle::from_json(&json).expect("parse failed");
        assert!(!parsed.reads_state, "absent field must parse as false");

        assert_eq!(parsed.name, bundle.name);
        assert_eq!(parsed.version, bundle.version);
        assert_eq!(parsed.target_vm, bundle.target_vm);
        assert_eq!(parsed.target_os, bundle.target_os);
        assert_eq!(parsed.entry_point, bundle.entry_point);
        assert_eq!(parsed.source_hash, bundle.source_hash);
        assert_eq!(parsed.cost.padded_height, bundle.cost.padded_height);
        assert_eq!(
            parsed.cost.estimated_proving_ns,
            bundle.cost.estimated_proving_ns
        );
        // Assembly contains newlines — verify escape roundtrip
        assert_eq!(parsed.assembly, bundle.assembly);
    }

    #[test]
    fn bundle_no_os() {
        let mut bundle = sample_bundle();
        bundle.target_os = None;
        let json = bundle.to_json();
        let parsed = ProgramBundle::from_json(&json).expect("parse failed");
        assert_eq!(parsed.target_os, None);
    }

    #[test]
    fn bundle_json_contains_assembly() {
        let bundle = sample_bundle();
        let json = bundle.to_json();
        assert!(json.contains("\"assembly\""));
        assert!(json.contains("push 42"));
    }
}
