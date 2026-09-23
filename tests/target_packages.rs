//! Ownership and resource identity must survive installed and library use.
use std::{collections::BTreeMap, fs};
use trident::{
    target::{RuntimeCapabilities, TargetPackage, TerrainConfig},
    CompileOptions,
};

fn package() -> TargetPackage {
    let terrain = TerrainConfig::nox();
    TargetPackage {
        intrinsic_abis: Default::default(),
        schema_version: 1,
        compiler_api: trident::COMPILER_API,
        owner: "joy".into(),
        version: "test".into(),
        intrinsics: terrain.supported_intrinsics(),
        terrain,
        union: None,
        states: Vec::new(),
        modules: BTreeMap::new(),
        module_hashes: BTreeMap::new(),
        instructions: Vec::new(),
        runtime: RuntimeCapabilities {
            run: true,
            prove: false,
            verify: false,
            deploy: false,
            proof_formats: Vec::new(),
            restrictions: Vec::new(),
        },
    }
    .seal()
    .unwrap()
}

#[test]
fn module_bytes_versions_and_abi_are_checked() {
    let mut valid = package();
    valid.modules.insert(
        "vm.nox.value".into(),
        "module vm.nox.value\npub fn get() -> Field { 7 }\n".into(),
    );
    let valid = valid.seal().unwrap();
    let mut changed = valid.clone();
    changed
        .modules
        .get_mut("vm.nox.value")
        .unwrap()
        .push_str("// changed\n");
    assert!(changed.validate().is_err());
    for unsupported in [0, 1, 2, trident::COMPILER_API + 1] {
        let mut changed = valid.clone();
        changed.compiler_api = unsupported;
        assert!(changed.validate().is_err());
    }
    let mut changed = valid.clone();
    changed.terrain.output_extension = "./escape".into();
    assert!(changed.validate().is_err());
    let mut changed = valid.clone();
    changed
        .modules
        .insert("std.target".into(), "module std.target".into());
    assert!(changed.seal().is_err());
    let mut changed = valid.clone();
    changed.version = "different".into();
    assert_ne!(
        changed.compilation_hash().unwrap(),
        valid.compilation_hash().unwrap()
    );
}

#[test]
fn source_identity_binds_effective_dependencies_and_selected_abi() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.tri");
    fs::write(
        &path,
        "program importing\nuse values\nfn main() -> Field { values.get() }\n",
    )
    .unwrap();
    let mut options = CompileOptions::default().with_package(package()).unwrap();
    options.module_sources.insert(
        "values".into(),
        "module values\npub fn get() -> Field { 7 }\n".into(),
    );
    let first = trident::compile_to_bundle(&path, &options).unwrap();
    options.module_sources.insert(
        "values".into(),
        "module values\npub fn get() -> Field { 8 }\n".into(),
    );
    let second = trident::compile_to_bundle(&path, &options).unwrap();
    assert_ne!(first.source_hash, second.source_hash);
    assert_ne!(first.assembly, second.assembly);
    options.target_config.digest_width = 5;
    assert!(trident::compile_to_bundle(&path, &options).is_err());
}

#[test]
fn ambient_library_tree_cannot_replace_packaged_module() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.tri");
    fs::create_dir_all(dir.path().join("vm/core")).unwrap();
    fs::write(
        dir.path().join("vm/core/field.tri"),
        "module vm.core.field\npub fn add(a: Field, b: Field) -> Field { 999 }\n",
    )
    .unwrap();
    fs::write(
        &path,
        "program arithmetic\nuse vm.core.field\nfn main() -> Field { field.add(2, 3) }\n",
    )
    .unwrap();
    let result = trident::compile_to_bundle(&path, &CompileOptions::default()).unwrap();
    fs::write(dir.path().join("vm/core/field.tri"), "invalid source").unwrap();
    let repeat = trident::compile_to_bundle(&path, &CompileOptions::default()).unwrap();
    assert_eq!(result, repeat);
}

#[test]
fn unavailable_or_mismatched_intrinsics_fail_during_checking() {
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("main.tri");
    fs::write(&entry, "program checking\nuse vm.fixture\nfn main() {}\n").unwrap();
    for source in [
        "module vm.fixture\n#[intrinsic(sponge_init)] pub fn start()\n",
        "module vm.fixture\n#[intrinsic(hash)] pub fn wrong(x: Field) -> Digest\n",
    ] {
        if source.contains("sponge_init") {
            fs::write(
                &entry,
                "program checking\nuse vm.fixture\nfn main() { fixture.start() }\n",
            )
            .unwrap();
        } else {
            fs::write(&entry, "program checking\nuse vm.fixture\nfn main() {}\n").unwrap();
        }
        let mut options = CompileOptions::default();
        options
            .module_sources
            .insert("vm.fixture".into(), source.into());
        let error = trident::check_project_with_options(&entry, &options).unwrap_err();
        assert!(
            error.iter().any(|e| e.message.contains("intrinsic")),
            "{error:?}"
        );
    }
    fs::write(
        &entry,
        "program no_tip5\nuse vm.triton.hash\nfn main() {}\n",
    )
    .unwrap();
    assert!(trident::check_project_with_options(&entry, &CompileOptions::default()).is_err());
}

#[test]
fn default_check_uses_nox_input_and_output_semantics() {
    assert!(trident::check_silent(
        "program good\nfn main(x: Field) -> Field { x + 1 }",
        "good.tri"
    )
    .is_ok());
    assert!(
        trident::check_silent("program stream\nfn main() { pub_write(1) }", "bad.tri").is_err()
    );
}

#[test]
fn package_resources_cannot_disguise_programs_or_other_unions() {
    for (key, source) in [
        (
            "vm.nox.value",
            "module vm.nox.other\npub fn get() -> Field { 7 }\n",
        ),
        ("vm.nox.value", "program vm.nox.value\nfn main() {}\n"),
        (
            "os.neptune.value",
            "module os.neptune.value\npub fn get() -> Field { 7 }\n",
        ),
    ] {
        let mut invalid = package();
        invalid.modules.insert(key.into(), source.into());
        assert!(invalid.seal().is_err(), "accepted {key}: {source}");
    }
}
