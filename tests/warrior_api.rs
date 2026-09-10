// ---
// tags: trident, rust, test
// crystal-type: source
// crystal-domain: comp
// ---
//! The warrior API contract, executable.
//!
//! `reference/warrior-api.md` promises a surface that warriors (joy,
//! trisha) may depend on, available **without default features**. This
//! test touches every path that document names, so the promise breaks
//! here — loudly, in CI — rather than in a warrior's build.
//!
//! Run both ways:
//! ```text
//! cargo test --test warrior_api
//! cargo test --test warrior_api --no-default-features
//! ```

use std::path::Path;

use trident::runtime::{
    Deployer, ExecutionResult, Guesser, ProgramBundle, ProgramInput, Prover, Runner, Verifier,
};
use trident::target::{Arch, TerrainConfig};
use trident::CompileOptions;

/// Every documented compilation entry point exists with the documented
/// shape. Typed, not called: the point is the signature.
#[allow(dead_code)]
fn compilation_surface_is_typed_as_documented() {
    let _bundle: fn(&Path, &CompileOptions) -> Result<ProgramBundle, Vec<trident::Diagnostic>> =
        trident::compile_to_bundle;
    let _tir: fn(
        &str,
        &str,
        &CompileOptions,
    ) -> Result<Vec<trident::tir::TIROp>, Vec<trident::Diagnostic>> = trident::build_tir;
    let _tir_project: fn(
        &Path,
        &CompileOptions,
    ) -> Result<Vec<trident::tir::TIROp>, Vec<trident::Diagnostic>> = trident::build_tir_project;
}

#[test]
fn compile_options_default_carries_a_terrain() {
    let options = CompileOptions::default();
    assert!(!options.target_config.name.is_empty());
    assert!(!options.profile.is_empty());
}

#[test]
fn both_built_in_terrains_resolve_without_a_target_toml() {
    let nox = TerrainConfig::nox();
    let triton = TerrainConfig::triton();
    assert_eq!(nox.name, "nox");
    assert_eq!(triton.name, "triton");
    assert_eq!(nox.architecture, Arch::Tree);
    assert_eq!(triton.architecture, Arch::Stack);
}

#[test]
fn a_warrior_can_build_tir_for_a_stack_terrain() {
    let source = "program w\n\nfn main() -> Field {\n    let a: Field = 6\n    a * 7\n}\n";
    let mut options = CompileOptions::default();
    options.target_config = TerrainConfig::triton();
    let ir = trident::build_tir(source, "w.tri", &options).expect("TIR for a stack terrain");
    assert!(!ir.is_empty(), "a non-empty program lowers to non-empty TIR");
}

#[test]
fn content_hash_is_stable_and_hex_round_trips() {
    let digest = trident::hash::content_hash_bytes(b"warrior");
    let hex = trident::hash::ContentHash(digest).to_hex();
    assert_eq!(hex.len(), 64);
    assert_eq!(
        trident::hash::ContentHash::from_hex(&hex).map(|h| h.0),
        Some(digest)
    );
    assert_eq!(trident::hash::content_hash_bytes(b"warrior"), digest);
}

#[test]
fn proof_sizing_helpers_are_reachable() {
    let padded = trident::field::proof::padded_height(1000);
    assert!(padded >= 1000 && padded.is_power_of_two());
}

#[test]
fn bundle_json_round_trip_keeps_what_the_reference_promises() {
    let bundle = ProgramBundle {
        name: "w".into(),
        version: "0.1.0".into(),
        target_vm: "triton".into(),
        target_os: None,
        assembly: "halt".into(),
        entry_point: "w__main".into(),
        functions: Vec::new(),
        cost: trident::runtime::artifact::BundleCost {
            table_values: Vec::new(),
            table_names: Vec::new(),
            padded_height: 256,
            estimated_proving_ns: 1,
        },
        source_hash: "ab".repeat(32),
        reads_state: true,
    };
    let back = ProgramBundle::from_json(&bundle.to_json()).expect("bundle round trip");
    assert_eq!(back.name, bundle.name);
    assert_eq!(back.assembly, bundle.assembly);
    assert_eq!(back.target_vm, bundle.target_vm);
    assert_eq!(back.entry_point, bundle.entry_point);
    assert_eq!(back.source_hash, bundle.source_hash);
    assert_eq!(back.reads_state, bundle.reads_state);
    assert_eq!(back.cost.padded_height, bundle.cost.padded_height);
}

#[test]
fn an_unknown_json_key_does_not_break_a_warrior() {
    let json = r#"{"name":"w","version":"0.1.0","target_vm":"nox","entry_point":"w__main",
        "source_hash":"00","assembly":"[1 5]","future_field":"a later trident wrote this"}"#;
    let bundle = ProgramBundle::from_json(json).expect("unknown keys are ignored");
    assert_eq!(bundle.name, "w");
    assert_eq!(bundle.assembly, "[1 5]");
}

/// The five warrior traits exist and are implementable outside the core.
/// A warrior implements the ones its battlefield supports.
struct StubWarrior;

impl Runner for StubWarrior {
    fn run(&self, _b: &ProgramBundle, _i: &ProgramInput) -> Result<ExecutionResult, String> {
        Err("stub".into())
    }
}

impl Prover for StubWarrior {
    fn prove(
        &self,
        _b: &ProgramBundle,
        _i: &ProgramInput,
    ) -> Result<trident::runtime::ProofData, String> {
        Err("stub".into())
    }
}

impl Verifier for StubWarrior {
    fn verify(&self, _p: &trident::runtime::ProofData) -> Result<bool, String> {
        Err("stub".into())
    }
}

impl Guesser for StubWarrior {
    fn guess(
        &self,
        _b: &ProgramBundle,
        _i: &ProgramInput,
        _difficulty: u64,
        _max_attempts: u64,
    ) -> Result<trident::runtime::GuessResult, String> {
        Err("stub".into())
    }
}

impl Deployer for StubWarrior {
    fn deploy(
        &self,
        _b: &ProgramBundle,
        _proof: Option<&trident::runtime::ProofData>,
    ) -> Result<String, String> {
        Err("stub".into())
    }
}

#[test]
fn the_warrior_traits_are_implementable_from_outside() {
    let w = StubWarrior;
    let input = ProgramInput {
        public: vec![1],
        secret: vec![],
        digests: vec![],
    };
    let bundle = ProgramBundle {
        name: "w".into(),
        version: String::new(),
        target_vm: "nox".into(),
        target_os: None,
        assembly: "[1 5]".into(),
        entry_point: String::new(),
        functions: Vec::new(),
        cost: trident::runtime::artifact::BundleCost {
            table_values: Vec::new(),
            table_names: Vec::new(),
            padded_height: 0,
            estimated_proving_ns: 0,
        },
        source_hash: String::new(),
        reads_state: false,
    };
    assert!(w.run(&bundle, &input).is_err(), "the stub refuses, by design");
    let _: &dyn Guesser = &w as &dyn Guesser;
    let _: &dyn Deployer = &w as &dyn Deployer;
}
