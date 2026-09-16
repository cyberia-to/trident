//! Installed compiler packaging must preserve target, dependency and artifact identity.
#![cfg(unix)]
mod support;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};

fn invoke(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_trident"))
        .args(args)
        .current_dir(dir)
        .env("PATH", dir)
        .env("TRIDENT_TARGET_PACKAGES", dir)
        .output()
        .unwrap()
}
fn require(output: Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn manifest(dir: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(dir.join("demo.deploy/manifest.json")).unwrap()).unwrap()
}
fn project(dir: &Path, target: &str, source: &str) {
    fs::write(dir.join("trident.toml"), format!("[project]\nname = \"demo\"\nversion = \"0.1.0\"\nentry = \"main.tri\"\ntarget = \"{target}\"\n")).unwrap();
    fs::write(dir.join("main.tri"), source).unwrap();
}
fn warrior(dir: &Path) {
    // Stub compilation only: verifies routing and writes a real requested file.
    // These tests never claim that the stub implements Triton proving.
    let path = dir.join("trisha");
    fs::write(
        &path,
        r#"#!/bin/sh
if [ "$1" = describe ]; then
    descriptor="$3.json"
    if [ -f provider.json ]; then descriptor=provider.json; fi
    while IFS= read -r line || [ -n "$line" ]; do printf '%s\n' "$line"; done < "$descriptor"
    exit 0
fi
printf '%s\n' "$@" > warrior.args
if [ -f fail-build ]; then exit 7; fi
while [ "$#" -gt 0 ]; do
    if [ "$1" = '-o' ]; then shift; printf 'halt\n' > "$1"; exit 0; fi
    shift
done
exit 1
"#,
    )
    .unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn nox_package_identity_tracks_imports_and_declares_nox_filename() {
    let dir = tempfile::tempdir().unwrap();
    project(
        dir.path(),
        "nox",
        "program demo\nuse calc\nfn main() -> Field { calc.value() }\n",
    );
    fs::write(
        dir.path().join("calc.tri"),
        "module calc\npub fn value() -> Field { 42 }\n",
    )
    .unwrap();
    require(invoke(dir.path(), &["package", "."]));
    assert!(!invoke(dir.path(), &["package", ".", "--state", "mainnet"])
        .status
        .success());
    let first = manifest(dir.path());
    assert_eq!(first["program_file"], "program.nox");
    assert_eq!(first["target"]["vm"], "nox");
    assert!(!dir.path().join("demo.deploy/program.tasm").exists());
    assert!(trident::deploy::load_artifact(&dir.path().join("demo.deploy")).is_ok());
    fs::write(
        dir.path().join("calc.tri"),
        "module calc\npub fn value() -> Field { 43 }\n",
    )
    .unwrap();
    require(invoke(dir.path(), &["package", "."]));
    let second = manifest(dir.path());
    assert_ne!(first["source_hash"], second["source_hash"]);
    assert_ne!(first["program_digest"], second["program_digest"]);
    require(invoke(dir.path(), &["deploy", "demo.deploy", "--dry-run"]));
    assert!(!invoke(
        dir.path(),
        &["deploy", "demo.deploy", "--dry-run", "--target", "triton"]
    )
    .status
    .success());
    let mut bad = second;
    bad["program_file"] = "../outside.nox".into();
    fs::write(
        dir.path().join("demo.deploy/manifest.json"),
        serde_json::to_vec(&bad).unwrap(),
    )
    .unwrap();
    assert!(!invoke(dir.path(), &["deploy", "demo.deploy", "--dry-run"])
        .status
        .success());
}

#[test]
fn project_foreign_target_delegates_build_and_keeps_cost_unknown() {
    let dir = tempfile::tempdir().unwrap();
    project(
        dir.path(),
        "triton",
        "program demo\nfn main() -> Field { 42 }\n",
    );
    support::write_triton_package(dir.path());
    warrior(dir.path());
    require(invoke(dir.path(), &["package", "."]));
    let foreign = manifest(dir.path());
    assert_eq!(foreign["program_file"], "program.tasm");
    assert_eq!(foreign["target"]["vm"], "triton");
    assert!(foreign["cost"]["padded_height"].is_null());
    let args = fs::read_to_string(dir.path().join("warrior.args")).unwrap();
    assert!(args.starts_with("build\n"));
    assert!(args.contains("--target\ntriton\n"));
    assert!(args.contains("-o\n"));
    assert!(!dir.path().join("demo.tasm").exists());
    require(invoke(dir.path(), &["package", ".", "--target", "nox"]));
    let nox = manifest(dir.path());
    assert_eq!(nox["program_file"], "program.nox");
    assert_ne!(foreign["source_hash"], nox["source_hash"]);
}

#[test]
fn neptune_package_preserves_union_and_embedded_sdk_identity() {
    let dir = tempfile::tempdir().unwrap();
    project(
        dir.path(),
        "neptune",
        "program demo\nuse os.neptune.fixture\nfn main() -> Field { fixture.value() }\n",
    );
    let mut package = support::triton_package();
    package.union = Some(trident::target::UnionConfig {
        name: "neptune".into(),
        display_name: "Fixture Neptune".into(),
        vm: "triton".into(),
        binding_prefix: "os.neptune".into(),
        account_model: "utxo".into(),
        storage_model: "fixture".into(),
        transaction_model: "fixture".into(),
    });
    package.modules.insert(
        "os.neptune.fixture".into(),
        "module os.neptune.fixture\npub fn value() -> Field { 42 }\n".into(),
    );
    fs::write(
        dir.path().join("neptune.json"),
        serde_json::to_vec(&package.clone().seal().unwrap()).unwrap(),
    )
    .unwrap();
    warrior(dir.path());
    // Registry publication cannot turn a state flag into a chain operation.
    // Reject it before invoking the compiler warrior or creating an artifact.
    let rejected = invoke(
        dir.path(),
        &["deploy", ".", "--state", "mainnet", "--dry-run"],
    );
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr)
        .contains("registry artifact publication does not apply a chain state selection"));
    assert!(!dir.path().join("warrior.args").exists());
    assert!(!dir.path().join("demo.deploy").exists());
    require(invoke(dir.path(), &["package", "."]));
    assert!(!invoke(dir.path(), &["package", ".", "--state", "mainnet"])
        .status
        .success());
    let first = manifest(dir.path());
    assert_eq!(first["target"]["os"], "neptune");
    assert!(fs::read_to_string(dir.path().join("warrior.args"))
        .unwrap()
        .contains("--target\nneptune\n"));
    package.modules.insert(
        "os.neptune.fixture".into(),
        "module os.neptune.fixture\npub fn value() -> Field { 43 }\n".into(),
    );
    fs::write(
        dir.path().join("neptune.json"),
        serde_json::to_vec(&package.seal().unwrap()).unwrap(),
    )
    .unwrap();
    require(invoke(dir.path(), &["package", "."]));
    assert_ne!(first["source_hash"], manifest(dir.path())["source_hash"]);
}

#[test]
fn locked_package_must_match_the_actual_warrior_before_build() {
    let dir = tempfile::tempdir().unwrap();
    project(
        dir.path(),
        "triton",
        "program demo\nfn main() -> Field { 42 }\n",
    );
    support::write_triton_package(dir.path());
    warrior(dir.path());
    let mut installed = support::triton_package();
    installed.version = "other-provider-version".into();
    fs::write(
        dir.path().join("provider.json"),
        serde_json::to_vec(&installed.seal().unwrap()).unwrap(),
    )
    .unwrap();
    let output = invoke(dir.path(), &["package", "."]);
    assert!(!output.status.success());
    assert!(
        !dir.path().join("warrior.args").exists(),
        "mismatch must fail before building"
    );
    assert!(!dir.path().join("demo.deploy").exists());
}

#[test]
fn failed_foreign_build_cleans_temporary_output_directory() {
    let dir = tempfile::tempdir().unwrap();
    project(
        dir.path(),
        "triton",
        "program demo\nfn main() -> Field { 42 }\n",
    );
    support::write_triton_package(dir.path());
    warrior(dir.path());
    fs::write(dir.path().join("fail-build"), "").unwrap();
    let output = invoke(dir.path(), &["package", "."]);
    assert!(!output.status.success());
    let args = fs::read_to_string(dir.path().join("warrior.args")).unwrap();
    let lines: Vec<_> = args.lines().collect();
    let at = lines.iter().position(|arg| *arg == "-o").unwrap();
    let output_path = Path::new(lines[at + 1]);
    assert!(
        !output_path.parent().unwrap().exists(),
        "temporary build directory leaked"
    );
    assert!(!dir.path().join("demo.deploy").exists());
}

#[test]
fn packaged_entry_matches_the_cfg_selected_public_function() {
    let dir = tempfile::tempdir().unwrap();
    project(dir.path(), "nox", "module example\nfn helper() -> Field { 99 }\n#[cfg(debug)] pub fn inactive() -> Field { 1 }\npub fn entry() -> Field { helper() }\n");
    require(invoke(
        dir.path(),
        &["package", ".", "--profile", "release"],
    ));
    let packaged = manifest(dir.path());
    assert_eq!(packaged["entry_point"], "entry");
    assert!(!packaged["functions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["name"] == "inactive"));
}
