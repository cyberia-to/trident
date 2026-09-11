//! Single-source APIs enforce the same ABI identity and entry requirements as projects.
use trident::CompileOptions;

fn nox_package() -> trident::target::TargetPackage {
    let terrain = trident::target::TerrainConfig::nox();
    let package = trident::target::TargetPackage {
        schema_version: 1,
        compiler_api: 1,
        owner: "joy".into(),
        version: "fixture".into(),
        intrinsics: terrain.supported_intrinsics(),
        terrain,
        union: None,
        states: vec![],
        modules: Default::default(),
        module_hashes: Default::default(),
        instructions: vec![],
        runtime: trident::target::RuntimeCapabilities {
            run: true,
            prove: false,
            verify: false,
            deploy: false,
            proof_formats: vec![],
            restrictions: vec![],
        },
    }
    .seal()
    .unwrap();
    package
}

#[test]
fn single_source_apis_reject_mutated_package_abi() {
    let mut options = CompileOptions::default()
        .with_package(nox_package())
        .unwrap();
    options.target_config.digest_width += 1;
    let source = "program valid\nfn main() -> Field { 7 }";
    assert!(trident::compile_with_options(source, "valid.tri", &options).is_err());
    assert!(trident::build_tir(source, "valid.tri", &options).is_err());
}

#[test]
fn module_entry_checks_transitive_requirements_before_lowering() {
    let source = "module entry\nfn private_helper() -> Field { ram_read(0) }\npub fn run() -> Field { private_helper() }";
    let options = CompileOptions::default();
    let compiled = trident::compile_with_options(source, "entry.tri", &options).unwrap_err();
    let tir = trident::build_tir(source, "entry.tri", &options).unwrap_err();
    for errors in [compiled, tir] {
        assert!(
            errors.iter().any(|error| error
                .message
                .contains("entry 'run' requires intrinsic 'ram_read'")),
            "{errors:?}"
        );
    }
}

#[test]
fn cfg_selected_module_entry_keeps_unused_requirements_out() {
    let source = "module entry\n#[cfg(release)]\npub fn unavailable() -> Field { ram_read(0) }\npub fn selected() -> Field { 7 }\npub fn unused() -> Field { ram_read(0) }";
    let options = CompileOptions::for_profile("debug");
    assert!(trident::compile_with_options(source, "entry.tri", &options).is_ok());
    assert!(trident::build_tir(source, "entry.tri", &options).is_ok());
    let options = CompileOptions::for_profile("release");
    assert!(trident::compile_with_options(source, "entry.tri", &options).is_err());
    assert!(trident::build_tir(source, "entry.tri", &options).is_err());
}

#[test]
fn a_sealed_package_cannot_redefine_the_reference_nox_abi() {
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("main.tri");
    let source = "program reference\nfn main() -> Field { 7 }";
    std::fs::write(&entry, source).unwrap();
    for changed in ["digest", "hash", "field", "architecture"] {
        let mut package = nox_package();
        match changed {
            "digest" => package.terrain.digest_width += 1,
            "hash" => package.terrain.hash_rate += 1,
            "field" => package.terrain.field_limbs += 1,
            "architecture" => package.terrain.architecture = trident::target::Arch::Stack,
            _ => unreachable!(),
        }
        // Structurally valid and internally consistent does not make this the nox ABI.
        let options = CompileOptions::default()
            .with_package(package.seal().unwrap())
            .unwrap();
        let errors = [
            trident::compile_with_options(source, "main.tri", &options).unwrap_err(),
            trident::build_tir(source, "main.tri", &options).unwrap_err(),
            trident::compile_to_bundle(&entry, &options).unwrap_err(),
            trident::check_project_with_options(&entry, &options).unwrap_err(),
        ];
        for errors in errors {
            assert!(
                errors
                    .iter()
                    .any(|error| error.message.contains("canonical nox ABI")),
                "{changed}: {errors:?}"
            );
        }
    }
}

#[test]
fn canonical_nox_packages_may_restrict_available_capabilities() {
    let mut package = nox_package();
    package.intrinsics.clear();
    let options = CompileOptions::default()
        .with_package(package.seal().unwrap())
        .unwrap();
    assert!(trident::compile_with_options(
        "program plain\nfn main() -> Field { 7 }",
        "plain.tri",
        &options
    )
    .is_ok());
    let arguments = vec!["7"; options.target_config.hash_rate as usize].join(", ");
    let source = format!("program needs_hash\nfn main() -> Digest {{ hash({arguments}) }}");
    let errors = trident::compile_with_options(&source, "hash.tri", &options).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("requires intrinsic 'hash'")));
}
