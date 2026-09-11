//! Build and cost reporting consume the same manifest, dependencies and entry.
use std::{fs, process::Command};

#[test]
fn build_and_costs_use_manifest_entry_path_dependencies_and_profile() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("vendor")).unwrap();
    fs::write(dir.path().join("trident.toml"), "[project]\nname = \"fixture\"\nentry = \"entry.tri\"\ntarget = \"nox\"\n[targets.debug]\nflags = [\"custom\"]\n[dependencies]\nhelper = { path = \"vendor\" }\n").unwrap();
    fs::write(
        dir.path().join("entry.tri"),
        "program fixture\nuse helper\nfn main() -> Field { helper.value() }\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("main.tri"),
        "invalid source must never be selected",
    )
    .unwrap();
    fs::write(
        dir.path().join("vendor/helper.tri"),
        "module helper\n#[cfg(custom)]\npub fn value() -> Field { 7 }\n",
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_trident"))
        .arg("build")
        .arg(dir.path())
        .arg("--costs")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(result.status.success(), "{stderr}");
    let (entry, options) =
        trident::source_options(dir.path(), &trident::CompileOptions::default()).unwrap();
    let expected = trident::compile_project_with_options(&entry, &options).unwrap();
    assert_eq!(
        fs::read_to_string(dir.path().join("fixture.nox")).unwrap(),
        expected
    );
    let cost = trident::nox_cost_project(&entry, &options)
        .unwrap()
        .format_report();
    assert!(stderr.contains(&cost), "{stderr}");
}
